#!/usr/bin/env python3
"""mockServer 的自检：起一个服务实例，把你关心的几条链路都打一遍。

    python selftest.py            # 默认在随机端口上跑一遍
    python selftest.py --verbose  # 顺便把服务端日志也打出来

它检查：

  * `GET /` 的说明页
  * `POST /v1/chat/completions` 在 url / b64 / mixed 三种模式下，
    images / audios / files 三类附件的字段形状（`{kind}_url.url`）
  * `POST /api/chat`（Ollama 形状）
  * `GET /media/*` 拿到的 png / wav 是不是真的能解析
  * `--count 2` 和 `--delay` 是否生效
"""

from __future__ import annotations

import argparse
import json
import socket
import struct
import subprocess
import sys
import time
import urllib.error
import urllib.request
import zlib
from pathlib import Path

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

HERE = Path(__file__).resolve().parent
SERVER = HERE / "mockServer.py"

checks = 0
failures = 0


def check(ok: bool, name: str, detail: str = "") -> None:
    global checks, failures
    checks += 1
    if ok:
        print(f"  [ok] {name}")
        return
    failures += 1
    print(f"  [失败] {name}" + (f"：{detail}" if detail else ""))


def free_port() -> int:
    with socket.socket() as sock:
        sock.bind(("127.0.0.1", 0))
        return sock.getsockname()[1]


def start_server(port: int, *extra: str) -> subprocess.Popen:
    process = subprocess.Popen(
        [sys.executable, str(SERVER), "--port", str(port), *extra],
        cwd=HERE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    base = f"http://127.0.0.1:{port}"
    for _ in range(50):
        try:
            with urllib.request.urlopen(base + "/", timeout=0.5):
                return process
        except (urllib.error.URLError, ConnectionError):
            time.sleep(0.1)
    process.kill()
    raise SystemExit(f"服务没起来，端口 {port}")


def stop_server(process: subprocess.Popen, verbose: bool) -> None:
    process.terminate()
    try:
        out, err = process.communicate(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        return
    if verbose:
        print("---- 服务端 stdout ----")
        print(out.decode("utf-8", "replace"))
        if err:
            print("---- 服务端 stderr ----")
            print(err.decode("utf-8", "replace"))


def post_json(base: str, path: str, payload: dict) -> dict:
    request = urllib.request.Request(
        base + path,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=15) as response:
        return json.loads(response.read().decode("utf-8"))


def get_bytes(base: str, path: str) -> tuple[str, bytes]:
    with urllib.request.urlopen(base + path, timeout=15) as response:
        return response.headers.get("Content-Type", ""), response.read()


def png_size(data: bytes) -> tuple[int, int] | None:
    """读 PNG 的 IHDR，并顺手校验每个 chunk 的 CRC。"""
    if not data.startswith(b"\x89PNG\r\n\x1a\n"):
        return None
    offset = 8
    size = None
    while offset + 8 <= len(data):
        (length,) = struct.unpack(">I", data[offset : offset + 4])
        tag = data[offset + 4 : offset + 8]
        payload = data[offset + 8 : offset + 8 + length]
        (crc,) = struct.unpack(">I", data[offset + 8 + length : offset + 12 + length])
        if zlib.crc32(tag + payload) & 0xFFFFFFFF != crc:
            return None
        if tag == b"IHDR":
            size = struct.unpack(">II", payload[:8])
        offset += 12 + length
    return size


def wav_info(data: bytes) -> tuple[int, int, int] | None:
    """返回 (采样率, 位深, 声道)，顺便确认 RIFF/WAVE/data 都在。"""
    if len(data) < 44 or not data.startswith(b"RIFF") or data[8:12] != b"WAVE":
        return None
    rate, bits, channels = struct.unpack("<I", data[24:28])[0], struct.unpack("<H", data[34:36])[0], struct.unpack("<H", data[22:24])[0]
    if b"data" not in data:
        return None
    return rate, bits, channels


def attachment_value(item: dict, kind: str) -> str | None:
    pointer = item.get(f"{kind}_url")
    if isinstance(pointer, dict):
        return pointer.get("url")
    return item.get("url")


def check_payload(payload: dict, mode: str, expected_count: int) -> None:
    message = payload["choices"][0]["message"]
    check(bool(message.get("content")), "content 不为空", repr(message.get("content"))[:60])

    for kind in ("image", "audio", "file"):
        items = message.get(f"{kind}s") or []
        check(
            len(items) == expected_count,
            f"{kind}s 有 {expected_count} 个",
            f"实际 {len(items)}",
        )
        for item in items:
            value = attachment_value(item, kind)
            if not value:
                check(False, f"{kind}s 的 {kind}_url.url 存在")
                continue
            if mode == "url":
                check(
                    value.startswith("http://"),
                    f"{kind} 是 http 链接",
                    value[:40],
                )
            elif mode == "b64":
                check(
                    value.startswith("data:") and ";base64," in value,
                    f"{kind} 是 data URL",
                    value[:40],
                )
            else:  # mixed
                if kind == "image":
                    check(value.startswith("http://"), "mixed：图片是 http 链接", value[:40])
                else:
                    check(value.startswith("data:"), f"mixed：{kind} 是 data URL", value[:40])


def run_case(mode: str, count: int, delay: float, verbose: bool) -> None:
    print(f"\n=== 模式 {mode}，每类 {count} 个，延迟 {delay}s ===")
    port = free_port()
    process = start_server(port, "--mode", mode, "--count", str(count), "--delay", str(delay))
    base = f"http://127.0.0.1:{port}"
    try:
        status, body = get_bytes(base, "/")
        info = json.loads(body.decode("utf-8"))
        check(status.startswith("application/json"), "GET / 返回 JSON")
        check(info["mode"] == mode, "说明页里的模式正确", info.get("mode", ""))
        check(
            set(info["attachments"]["file"]) == {"test.txt"},
            "文件类附件选到了 test.txt",
            str(info["attachments"]["file"]),
        )

        started = time.monotonic()
        payload = post_json(
            base,
            "/v1/chat/completions",
            {"model": "mock", "messages": [{"role": "user", "content": "自检"}]},
        )
        elapsed = time.monotonic() - started
        check_payload(payload, mode, count)
        if delay > 0:
            check(elapsed >= delay, f"延迟 {delay}s 生效", f"只用了 {elapsed:.2f}s")

        ollama = post_json(
            base,
            "/api/chat",
            {"model": "mock", "messages": [{"role": "user", "content": "自检"}]},
        )
        check(
            bool(ollama.get("message", {}).get("content")),
            "Ollama 形状有 message.content",
        )
        check(ollama.get("done") is True, "Ollama 形状 done=true")

        content_type, png = get_bytes(base, "/media/test.png")
        check(content_type.startswith("image/png"), "test.png 的 Content-Type", content_type)
        size = png_size(png)
        check(size is not None, "test.png 是合法 PNG（CRC 全对）")
        check(size == (48, 48), "test.png 尺寸 48x48", str(size))

        content_type, wav = get_bytes(base, "/media/test.wav")
        check(content_type.startswith("audio/wav"), "test.wav 的 Content-Type", content_type)
        info_wav = wav_info(wav)
        check(info_wav is not None, "test.wav 是合法 WAV")
        if info_wav:
            check(info_wav[0] == 22050 and info_wav[1] == 16, "test.wav 22.05kHz/16bit", str(info_wav))
    finally:
        stop_server(process, verbose)


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="mockServer 自检")
    parser.add_argument("--verbose", action="store_true", help="同时打印服务端日志")
    parser.add_argument("--quick", action="store_true", help="只跑 mixed 模式")
    args = parser.parse_args(argv)

    if not SERVER.is_file():
        raise SystemExit(f"找不到 {SERVER}")

    modes = [("mixed", 1, 0.0)] if args.quick else [
        ("mixed", 1, 0.0),
        ("url", 2, 0.0),
        ("b64", 1, 0.0),
        ("mixed", 1, 0.5),
    ]
    for mode, count, delay in modes:
        run_case(mode, count, delay, args.verbose)

    print()
    if failures:
        print(f"失败 {failures} / {checks}")
        return 1
    print(f"Pass.（{checks} 项检查）")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
