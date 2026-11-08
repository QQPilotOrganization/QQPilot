# -*- coding: utf-8 -*-
"""
json_translator.py —— 纯标准库 JSON 翻译 CLI（llama.cpp + HyMT2）

用法示例：
    python json_translator.py strings.json -t 英语
    python json_translator.py strings.json -t 日语 -o out/strings.ja.json
    python json_translator.py strings.json -t 法语 --no-stream

说明：
  - 输入 JSON 顶层必须是 {"key": "value", ...} 形式；
  - 只翻译字符串类型的 value，其它类型原样保留；
  - 每个 value 独立请求，长文本会按 max-chars 分段后拼接；
  - 翻译过程逐字流式输出到 stderr（可 --no-stream 关闭）；
  - 结果写入新文件，不修改原文件。
  
  
  运行类似 llama-server -m Hy-MT2-1.8B-UD-Q4_K_XL.gguf --host 127.0.0.1 --port 12501
  
  
  I:\llamacpp\models\HyMT2\run.cmd
  
"""
import argparse
import json
import re
import sys
import urllib.error
import urllib.request
from pathlib import Path


# HyMT2 对话控制 token，作为 stop 序列，避免译文末尾混入标记
STOP_SEQUENCES = [
    "<｜hy_User｜>",
    "<｜hy_Assistant｜>",
    "<｜hy_end▁of▁sentence｜>",
]

_PROMPT_TMPL = "将以下文本翻译为{lang}，注意只需要输出翻译后的结果，不要额外解释：\n\n{text}"


# --------------------------------------------------------------------------
# 文本处理
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

        # 按中英文句末标点切句
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
# llama.cpp 流式请求（urllib 实现 SSE 解析）
# --------------------------------------------------------------------------
def iter_translate(
    segment: str,
    lang: str,
    base_url: str,
    model: str,
    timeout: float,
    temperature: float,
    max_tokens: int,
):
    """对单个分段发起流式请求，逐块 yield 译文文本。"""
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": _PROMPT_TMPL.format(lang=lang, text=segment)}],
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
        for raw_line in resp:  # HTTPResponse 支持逐行迭代
            line = raw_line.decode("utf-8", errors="ignore").strip()
            if not line:
                continue

            # 标准 SSE：data: {...}
            if line.startswith("data:"):
                data = line[5:].strip()
                if data == "[DONE]":
                    break
                try:
                    obj = json.loads(data)
                except json.JSONDecodeError:
                    continue
            else:
                # 兜底：服务端忽略 stream 时一次性返回完整 JSON
                try:
                    obj = json.loads(line)
                except json.JSONDecodeError:
                    continue

            for choice in obj.get("choices") or []:
                content = (choice.get("delta") or {}).get("content")
                if content:
                    yield content


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
    """翻译一个 value（自动分段），返回拼接后的完整译文。"""
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


# --------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------
def _default_output(input_path: Path, lang: str) -> Path:
    safe_lang = re.sub(r'[\\/:*?"<>|]+', "_", lang) or "out"
    return input_path.with_name(f"{input_path.stem}.{safe_lang}.json")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        description="使用 llama.cpp + HyMT2 翻译 JSON 中的 value（纯标准库）"
    )
    ap.add_argument("input", type=Path, help="输入 JSON 文件路径")
    ap.add_argument("-t", "--target", required=True, help="目标语言，如 英语 / 日语 / 法语")
    ap.add_argument("-o", "--output", type=Path, default=None,
                    help="输出 JSON 路径（默认：<原名>.<目标语言>.json）")
    ap.add_argument("--base-url", default="http://127.0.0.1:12501",
                    help="llama.cpp 服务地址（默认 http://127.0.0.1:12501）")
    ap.add_argument("--model", default="Hy-MT2",
                    help="模型名（llama.cpp 通常不校验，用于日志）")
    ap.add_argument("--max-chars", type=int, default=500,
                    help="单个请求的最大字符数（默认 500）")
    ap.add_argument("--timeout", type=float, default=300.0,
                    help="单次请求超时秒数（默认 300）")
    ap.add_argument("--temperature", type=float, default=0.0,
                    help="采样温度，翻译建议 0（默认 0.0）")
    ap.add_argument("--max-tokens", type=int, default=4096,
                    help="单次生成最大 token 数（默认 4096）")
    ap.add_argument("--retries", type=int, default=1,
                    help="网络错误时的重试次数（默认 1）")
    ap.add_argument("--no-stream", action="store_true",
                    help="关闭流式字符输出，只在每条结束后打印整句")
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
        print("错误：输入 JSON 顶层必须是对象 {\"key\": \"value\", ...}", file=sys.stderr)
        return 2

    output_path = args.output or _default_output(args.input, args.target)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    total = len(entries)
    translated: dict = {}
    stream = not args.no_stream
    failures = 0

    for idx, (key, value) in enumerate(entries.items(), start=1):
        # 非字符串 / 空串 → 原样保留
        if not isinstance(value, str) or not value.strip():
            translated[key] = value
            continue

        prefix = f"[{idx}/{total}] {key}: "
        sys.stderr.write(prefix)
        sys.stderr.flush()

        try:
            result = translate_value(
                value, args.target,
                base_url=args.base_url,
                model=args.model,
                timeout=args.timeout,
                temperature=args.temperature,
                max_tokens=args.max_tokens,
                max_chars=args.max_chars,
                stream=stream,
                retries=args.retries,
            )
        except Exception as e:  # noqa: BLE001
            failures += 1
            sys.stderr.write(f"失败：{e}\n")
            translated[key] = value  # 失败时回退为原文，保证输出结构完整
            continue

        if stream:
            sys.stderr.write("\n")
        else:
            sys.stderr.write(f"{result}\n")
        sys.stderr.flush()
        translated[key] = result

    output_path.write_text(
        json.dumps(translated, ensure_ascii=False, indent=args.indent) + "\n",
        encoding="utf-8",
    )

    print(f"\n完成：{total - failures}/{total} 条已翻译", file=sys.stderr)
    if failures:
        print(f"注意：有 {failures} 条失败，已保留原文，请检查服务或重试。", file=sys.stderr)
    print(f"输出文件：{output_path}", file=sys.stderr)
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    sys.exit(main())