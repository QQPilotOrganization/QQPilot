# -*- coding: utf-8 -*-
"""
json_translator.py —— 纯标准库 JSON 翻译 CLI（llama.cpp + Hy-MT3）

用法示例：
    python json_translator.py strings.json -t 英语
    python json_translator.py strings.json -t 日语 -o out/strings.ja.json
    python json_translator.py strings.json -t 法语 --mode json
    python json_translator.py strings.json -t 法语 --mode per-value

翻译模式（--mode）：
    auto        (默认) 先让模型直译整个 JSON，失败则回退到逐值
    json        只用 JSON 直译模式（失败即报错，便于观察模型表现）
    per-value   传统的逐条翻译（适合调试 / 超长文件 / 强制一致性）

说明：
  - 输入 JSON 顶层必须是 {"key": "value", ...} 形式；
  - JSON 直译模式内部会按 --json-chunk-chars 自动分块；
  - 响应会剥离 ``` 围栏、提取首个完整 JSON 对象；
  - 校验：键集合必须一致、每个键的顶层值类型必须一致；
  - 结果写入新文件，不修改原文件。

运行 llama-server 示例：
    llama-server -m Hy-MT3-1.8B-UD-Q4_K_M.gguf --host 127.0.0.1 --port 12501
"""
import argparse
import json
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path


# HyMT2/HyMT3 对话控制 token，作为 stop 序列，避免译文末尾混入标记
STOP_SEQUENCES = [
    "<｜hy_User｜>",
    "<｜hy_Assistant｜>",
    "<｜hy_end▁of▁sentence｜>",
]

# 逐值翻译提示词
_PROMPT_TMPL = "将以下文本翻译为{lang}，注意只需要输出翻译后的结果，不要额外解释：\n\n{text}"

# 整段 JSON 直译提示词 —— 使用 str.replace 注入，{{var}} 会原样传达给模型
_JSON_PROMPT_TMPL = """# 任务目标
将下方 JSON 中面向用户的字符串值翻译为{lang}。

# 严格约束
1. 结构锁定：绝对保持原有的 JSON 数据结构、缩进和层级完全不变。
2. 选择性翻译：仅翻译面向用户展示的可见文本内容。
3. 禁止修改：严禁翻译或更改任何代码标签、键名 (Key)、变量占位符（如 {{var}}、${var}、%s、%d 等）或代码属性。

# 输出要求
- 只输出翻译后的完整 JSON，不要任何解释、不要 Markdown 代码块、不要前后缀。
- 保持所有键名、键顺序、缩进（2 空格）和值类型（字符串 / 数字 / 布尔 / null / 数组 / 对象）不变。

# 数据输入
{json}
"""


def build_json_prompt(lang: str, json_text: str) -> str:
    """把提示词模板中的占位符替换掉；{{var}} 等花括号语法会原样保留。"""
    return _JSON_PROMPT_TMPL.replace("{lang}", lang).replace("{json}", json_text)


# --------------------------------------------------------------------------
# 文本分段（逐值模式用）
# --------------------------------------------------------------------------
def split_segments(text: str, max_chars: int) -> list[str]:
    """优先按空行切；单段过长再按句末标点切；最后硬切。"""
    if len(text) <= max_chars:
        return [text]

    segments: list[str] = []
    for block in [b.strip() for b in text.split("\n\n") if b.strip()]:
        if len(block) <= max_chars:
            segments.append(block)
            continue

        sentences, buf = [], ""
        for ch in block:
            buf += ch
            if ch in "。！？.!?":
                sentences.append(buf.strip())
                buf = ""
        if buf.strip():
            sentences.append(buf.strip())

        cur = ""
        for s in sentences:
            if len(cur) + len(s) <= max_chars:
                cur += s
            else:
                if cur:
                    segments.append(cur)
                cur = s
        if cur:
            segments.append(cur)

    return segments or [text]


# --------------------------------------------------------------------------
# 底层：SSE 流式请求
# --------------------------------------------------------------------------
def iter_completion(
    prompt: str,
    *,
    base_url: str,
    model: str,
    timeout: float,
    temperature: float,
    max_tokens: int,
):
    """对 llama.cpp /v1/chat/completions 发起流式请求，逐块 yield content。"""
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "stream": True,
        "temperature": temperature,
        "stop": STOP_SEQUENCES,
        "max_tokens": max_tokens,
    }
    req = urllib.request.Request(
        f"{base_url.rstrip('/')}/v1/chat/completions",
        data=json.dumps(payload, ensure_ascii=False).encode("utf-8"),
        headers={
            "Content-Type": "application/json; charset=utf-8",
            "Accept": "text/event-stream",
        },
        method="POST",
    )

    with urllib.request.urlopen(req, timeout=timeout) as resp:
        for raw_line in resp:
            line = raw_line.decode("utf-8", errors="ignore").strip()
            if not line:
                continue
            if line.startswith("data:"):
                data = line[5:].strip()
                if data == "[DONE]":
                    break
                try:
                    obj = json.loads(data)
                except json.JSONDecodeError:
                    continue
            else:
                # 服务端忽略 stream 时一次性返回完整 JSON
                try:
                    obj = json.loads(line)
                except json.JSONDecodeError:
                    continue

            for choice in obj.get("choices") or []:
                content = (choice.get("delta") or {}).get("content")
                if content:
                    yield content


def iter_translate(segment, lang, base_url, model, timeout, temperature, max_tokens):
    prompt = _PROMPT_TMPL.format(lang=lang, text=segment)
    yield from iter_completion(
        prompt,
        base_url=base_url,
        model=model,
        timeout=timeout,
        temperature=temperature,
        max_tokens=max_tokens,
    )


# --------------------------------------------------------------------------
# 逐值翻译（原模式）
# --------------------------------------------------------------------------
def translate_value(
    text: str,
    lang: str,
    *,
    base_url: str,
    model: str,
    timeout: float,
    temperature: float,
    max_tokens: int,
    max_chars: int,
    stream: bool,
    retries: int,
) -> str:
    parts: list[str] = []
    for seg in split_segments(text, max_chars):
        attempt = 0
        while True:
            try:
                for piece in iter_translate(seg, lang, base_url, model,
                                            timeout, temperature, max_tokens):
                    parts.append(piece)
                    if stream:
                        sys.stderr.write(piece)
                        sys.stderr.flush()
                break
            except (urllib.error.URLError, TimeoutError) as e:
                attempt += 1
                if attempt > retries:
                    raise RuntimeError(f"请求失败：{e}") from e
                sys.stderr.write(f"\n[重试 {attempt}/{retries}] {e}\n")
                sys.stderr.flush()
    return "".join(parts)


def translate_per_value(entries: dict, args, stream: bool) -> tuple[dict, int]:
    total = len(entries)
    translated: dict = {}
    failures = 0

    for idx, (key, value) in enumerate(entries.items(), start=1):
        if not isinstance(value, str) or not value.strip():
            translated[key] = value
            continue

        sys.stderr.write(f"[{idx}/{total}] {key}: ")
        sys.stderr.flush()
        try:
            result = translate_value(
                value, args.target,
                base_url=args.base_url, model=args.model,
                timeout=args.timeout, temperature=args.temperature,
                max_tokens=args.max_tokens, max_chars=args.max_chars,
                stream=stream, retries=args.retries,
            )
        except Exception as e:  # noqa: BLE001
            failures += 1
            sys.stderr.write(f"失败：{e}\n")
            translated[key] = value
            continue

        if stream:
            sys.stderr.write("\n")
        else:
            sys.stderr.write(f"{result}\n")
        sys.stderr.flush()
        translated[key] = result

    sys.stderr.write(f"\n逐值翻译：{total - failures}/{total} 条\n")
    return translated, failures


# --------------------------------------------------------------------------
# JSON 直译：响应清洗 + 结构校验
# --------------------------------------------------------------------------
def parse_json_object(text: str) -> dict | None:
    """从模型原始输出中提取第一个完整 JSON 对象。自动去掉 ``` 围栏。"""
    text = text.strip()
    if not text:
        return None

    # 剥 ```json ... ``` 围栏
    if text.startswith("```"):
        lines = text.splitlines()
        if lines and lines[0].startswith("```"):
            lines = lines[1:]
        if lines and lines[-1].strip() == "```":
            lines = lines[:-1]
        text = "\n".join(lines).strip()

    # 直接尝试
    try:
        obj = json.loads(text)
        if isinstance(obj, dict):
            return obj
    except json.JSONDecodeError:
        pass

    # 退化：从第一个 '{' 开始解析
    start = text.find("{")
    if start == -1:
        return None
    try:
        decoder = json.JSONDecoder()
        obj, _ = decoder.raw_decode(text, start)
        if isinstance(obj, dict):
            return obj
    except json.JSONDecodeError:
        pass
    return None


def _type_tag(v) -> str:
    if isinstance(v, bool):
        return "bool"
    if v is None:
        return "null"
    if isinstance(v, (int, float)):
        return "number"
    if isinstance(v, str):
        return "string"
    if isinstance(v, list):
        return "list"
    if isinstance(v, dict):
        return "dict"
    return type(v).__name__


def validate_structure(original: dict, translated: dict) -> bool:
    if set(original.keys()) != set(translated.keys()):
        return False
    for k, v in original.items():
        if _type_tag(v) != _type_tag(translated[k]):
            return False
    return True


# --------------------------------------------------------------------------
# JSON 直译：分块 & 单块翻译
# --------------------------------------------------------------------------
def chunk_entries(entries: dict, max_chars: int) -> list[dict]:
    """按 json.dumps 后的字符数切块，尽量保持条目完整。"""
    chunks: list[dict] = []
    current: dict = {}
    current_size = 2  # {}

    for k, v in entries.items():
        item_size = len(json.dumps({k: v}, ensure_ascii=False))

        # 单条就超过预算：独占一块
        if not current and item_size > max_chars:
            chunks.append({k: v})
            continue

        # 放不下：封块
        if current and current_size + item_size + 2 > max_chars:
            chunks.append(current)
            current = {}
            current_size = 2

        current[k] = v
        current_size += item_size + 2  # 逗号 + 换行

    if current:
        chunks.append(current)
    return chunks


def _translate_json_block(block: dict, args, show_stream: bool) -> dict | None:
    """单块 JSON 翻译，返回 dict 或 None（失败）。"""
    json_text = json.dumps(block, ensure_ascii=False, indent=2)
    prompt = build_json_prompt(args.target, json_text)

    for attempt in range(args.retries + 1):
        try:
            parts: list[str] = []
            for piece in iter_completion(
                prompt,
                base_url=args.base_url, model=args.model,
                timeout=args.timeout, temperature=args.temperature,
                max_tokens=args.max_tokens,
            ):
                parts.append(piece)
                if show_stream:
                    sys.stderr.write(piece)
                    sys.stderr.flush()
            raw = "".join(parts)
            if show_stream:
                sys.stderr.write("\n")
                sys.stderr.flush()

            parsed = parse_json_object(raw)
            if parsed is None:
                sys.stderr.write(f"[JSON 解析失败，重试 {attempt + 1}/{args.retries}]\n")
                continue
            if not validate_structure(block, parsed):
                sys.stderr.write(f"[JSON 结构校验失败，重试 {attempt + 1}/{args.retries}]\n")
                continue
            return parsed

        except (urllib.error.URLError, TimeoutError, ConnectionError) as e:
            sys.stderr.write(f"[网络错误，重试 {attempt + 1}/{args.retries}] {e}\n")
    return None


def try_json_mode(entries: dict, args, show_stream: bool) -> dict | None:
    """尝试用 JSON 直译模式翻译整个 entries，成功返回 dict，失败返回 None。"""
    total_size = len(json.dumps(entries, ensure_ascii=False))

    if total_size <= args.json_chunk_chars:
        sys.stderr.write(f"[JSON 模式] 整文件一次翻译（约 {total_size} 字符）\n")
        sys.stderr.flush()
        return _translate_json_block(entries, args=args, show_stream=show_stream)

    chunks = chunk_entries(entries, args.json_chunk_chars)
    sys.stderr.write(f"[JSON 模式] 分 {len(chunks)} 块翻译\n")
    sys.stderr.flush()

    results: dict = {}
    for i, chunk in enumerate(chunks, 1):
        sys.stderr.write(f"[{i}/{len(chunks)}] {len(chunk)} 项：")
        sys.stderr.flush()
        r = _translate_json_block(chunk, args=args, show_stream=False)
        if r is None:
            sys.stderr.write("失败\n")
            return None
        results.update(r)
        sys.stderr.write("OK\n")
        sys.stderr.flush()
    return results


# --------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------
def _default_output(input_path: Path, lang: str) -> Path:
    safe_lang = re.sub(r'[\\/:*?"<>|]+', "_", lang) or "out"
    return input_path.with_name(f"{input_path.stem}.{safe_lang}.json")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        description="使用 llama.cpp + Hy-MT3 翻译 JSON 中的 value（纯标准库）"
    )
    ap.add_argument("input", type=Path, help="输入 JSON 文件路径")
    ap.add_argument("-t", "--target", required=True, help="目标语言，如 英语 / 日语 / 法语")
    ap.add_argument("-o", "--output", type=Path, default=None,
                    help="输出 JSON 路径（默认：<原名>.<目标语言>.json）")
    ap.add_argument("--base-url", default="http://127.0.0.1:12501",
                    help="llama.cpp 服务地址（默认 http://127.0.0.1:12501）")
    ap.add_argument("--model", default="Hy-MT3",
                    help="模型名（llama.cpp 通常不校验，用于日志）")
    ap.add_argument("--mode", choices=["auto", "json", "per-value"], default="auto",
                    help="翻译模式：auto=优先JSON失败回退（默认）；json=只用JSON；per-value=逐条翻译")
    ap.add_argument("--json-chunk-chars", type=int, default=3000,
                    help="JSON 直译模式单块最大字符数（默认 3000）")
    ap.add_argument("--max-chars", type=int, default=500,
                    help="逐值模式单条最大字符数（默认 500）")
    ap.add_argument("--timeout", type=float, default=300.0,
                    help="单次请求超时秒数（默认 300）")
    ap.add_argument("--temperature", type=float, default=0.0,
                    help="采样温度，翻译建议 0（默认 0.0）")
    ap.add_argument("--max-tokens", type=int, default=8192,
                    help="单次生成最大 token 数（默认 8192）")
    ap.add_argument("--retries", type=int, default=1,
                    help="网络或解析失败时的重试次数（默认 1）")
    ap.add_argument("--no-stream", action="store_true",
                    help="关闭流式字符输出，只在每条结束后打印结果")
    ap.add_argument("--indent", type=int, default=2,
                    help="输出 JSON 缩进（默认 2）")
    args = ap.parse_args(argv)

    # 读取输入
    try:
        raw = args.input.read_text(encoding="utf-8")
        entries = json.loads(raw)
    except FileNotFoundError:
        print(f"错误：找不到文件 {args.input}", file=sys.stderr)
        return 2
    except json.JSONDecodeError as e:
        print(f"错误：输入不是合法 JSON：{e}", file=sys.stderr)
        return 2

    if not isinstance(entries, dict):
        print('错误：输入 JSON 顶层必须是对象 {"key": "value", ...}', file=sys.stderr)
        return 2

    output_path = args.output or _default_output(args.input, args.target)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    stream = not args.no_stream
    translated: dict | None = None
    failures = 0

    # ---- JSON 直译模式 ----
    if args.mode in ("auto", "json"):
        # 只有在显式 --mode json 时才把原始 JSON 流式打到 stderr（避免噪音）
        json_show_stream = stream and args.mode == "json"
        translated = try_json_mode(entries, args=args, show_stream=json_show_stream)

        if translated is not None:
            # 按输入顺序重排（防止模型重排键顺序）
            translated = {k: translated[k] for k in entries if k in translated}
            sys.stderr.write(f"[JSON 模式] 完成 {len(translated)}/{len(entries)} 条\n")
        else:
            if args.mode == "json":
                print("错误：JSON 模式翻译失败", file=sys.stderr)
                return 1
            print("JSON 模式失败，回退到逐值翻译...", file=sys.stderr)

    # ---- 逐值回退 ----
    if translated is None:
        translated, failures = translate_per_value(entries, args, stream)

    # ---- 写文件 ----
    output_path.write_text(
        json.dumps(translated, ensure_ascii=False, indent=args.indent) + "\n",
        encoding="utf-8",
    )

    print(f"\n完成：输出文件 {output_path}", file=sys.stderr)
    if failures:
        print(f"注意：有 {failures} 条失败，已保留原文，请检查服务或重试。", file=sys.stderr)
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    sys.exit(main())