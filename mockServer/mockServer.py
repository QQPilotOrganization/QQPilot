#!/usr/bin/env python3
"""QQPilot 的假 LLM 服务：用来测试"回复里带图片 / 音频 / 文件"这条链路。

只依赖 Python 标准库（3.8+），不联网、不装包，`Ctrl+C` 退出。

它提供三样东西：

  POST /v1/chat/completions   OpenAI 兼容的回复（qqpilot5 默认走这个）
  POST /api/chat              Ollama 形状的回复（forceollamaapi=true 时走这个）
  GET  /media/<名字>          附件本体：test.png / test.wav / test.txt（或 --media-dir 里的文件）
  GET  /                      一页 JSON 说明（当前模式、附件数量、要填的配置）

回复里会按 `--mode` 把三类附件塞进 `choices[0].message.{images,audios,files}`：

  url    三种都给 http(s) 链接 —— 测 qqpilot5 的"下载远程附件"（remote_* = true）
  b64    三种都给 data URL    —— 测本地解码，不碰网络
  mixed  图片给链接、音频给 data URL、文件给链接（默认，一次把两条路都走一遍）

用法示例：

    python mockServer.py                        # 127.0.0.1:8000，混合模式
    python mockServer.py --port 9000 --count 2  # 换端口，每类两个附件
    python mockServer.py --mode b64 --delay 1.5 # 全 data URL，顺便模拟慢响应
    python mockServer.py --media-dir .\\media    # 用自己准备的文件当附件

然后在 qqpilot5 的 config.ini 里填：

    modelname = mock
    server_url = http://127.0.0.1:8000/v1
    forceollamaapi = false
"""

from __future__ import annotations

import argparse
import base64
import json
import math
import struct
import sys
import threading
import time
import zlib
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

def say(text: str = "") -> None:
    """打印一行；控制台/管道编码不支持中文时退回 UTF-8 字节输出。

    服务本身绝不能因为"日志打不出来"而崩掉（管道重定向到 cp1252 时就会这样）。
    """
    try:
        print(text, flush=True)
        return
    except (UnicodeEncodeError, ValueError):
        pass
    try:
        buffer = getattr(sys.stdout, "buffer", None)
        if buffer is not None:
            buffer.write((text + "\n").encode("utf-8", "replace"))
            buffer.flush()
    except (OSError, ValueError):
        pass

def configure_stdio() -> None:
    """让中文能正常打印。

    Windows 控制台默认用本地代码页（这台机器是 cp1252），直接 print 中文会抛
    UnicodeEncodeError 把服务打挂。这里把 stdout/stderr 改成 UTF-8（出错也不崩），
    顺便把控制台输出代码页切到 65001。
    """
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, "reconfigure"):
            try:
                stream.reconfigure(encoding="utf-8", errors="replace")
            except (ValueError, OSError):
                pass
    if sys.platform == "win32":
        try:
            import ctypes

            ctypes.windll.kernel32.SetConsoleOutputCP(65001)
        except Exception:  # noqa: BLE001 - 控制台调不动也不影响服务
            pass

# 一页说明里用到的默认附件名
MEDIA_FILES = {
    "test.png": "image/png",
    "test.wav": "audio/wav",
    "test.txt": "text/plain; charset=utf-8",
}


# --------------------------------------------------------------------------
# 生成测试用的媒体文件（纯标准库）
# --------------------------------------------------------------------------

def make_png(width: int = 48, height: int = 48) -> bytes:
    """画一张 48x48 的测试图：蓝底上一个白框 + 一条红色对角线。

    尺寸刻意做小：这份数据经常以 base64 塞进响应里，太大只会让日志难读。
    """
    rows = bytearray()
    for y in range(height):
        rows.append(0)  # 每行的 filter byte
        for x in range(width):
            border = x < 3 or y < 3 or x >= width - 3 or y >= height - 3
            diagonal = abs((x * height) - (y * width)) < width * 2
            if border:
                rows += bytes((255, 255, 255))
            elif diagonal:
                rows += bytes((220, 60, 60))
            else:
                rows += bytes((40, 90, 200))

    def chunk(tag: bytes, payload: bytes) -> bytes:
        return (
            struct.pack(">I", len(payload))
            + tag
            + payload
            + struct.pack(">I", zlib.crc32(tag + payload) & 0xFFFFFFFF)
        )

    header = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)  # 8bit RGB
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(bytes(rows), 9))
        + chunk(b"IEND", b"")
    )


def make_wav(seconds: float = 0.3, frequency: float = 440.0) -> bytes:
    """一段 440Hz 的正弦音（音量不大），用来确认"真的能放出来"。"""
    rate = 22050
    samples = int(rate * seconds)
    amplitude = 0.25
    data = bytearray()
    for i in range(samples):
        value = int(32767 * amplitude * math.sin(2 * math.pi * frequency * i / rate))
        data += struct.pack("<h", value)

    byte_rate = rate * 2
    header = (
        b"RIFF"
        + struct.pack("<I", 36 + len(data))
        + b"WAVE"
        + b"fmt "
        + struct.pack("<IHHIIHH", 16, 1, 1, rate, byte_rate, 2, 16)
        + b"data"
        + struct.pack("<I", len(data))
    )
    return header + bytes(data)


def make_text() -> bytes:
    return (
        "这是 mockServer 生成的一个测试文件。\n"
        "如果你在 QQ 聊天窗口里看到它被当成文件发出去，说明文件上传链路是通的。\n"
    ).encode("utf-8")


def build_media(args: argparse.Namespace) -> dict[str, tuple[bytes, str]]:
    """返回 {文件名: (内容, Content-Type)}。

    给了 `--media-dir` 就用那个目录里的文件（按扩展名判断类型），
    否则用现场生成的 png/wav/txt。
    """
    if args.media_dir:
        directory = Path(args.media_dir)
        if not directory.is_dir():
            raise SystemExit(f"--media-dir 不是目录: {directory}")
        media: dict[str, tuple[bytes, str]] = {}
        for path in sorted(directory.iterdir()):
            if not path.is_file():
                continue
            suffix = path.suffix.lower()
            mime = {
                ".png": "image/png",
                ".jpg": "image/jpeg",
                ".jpeg": "image/jpeg",
                ".gif": "image/gif",
                ".webp": "image/webp",
                ".wav": "audio/wav",
                ".mp3": "audio/mpeg",
                ".flac": "audio/flac",
                ".ogg": "audio/ogg",
                ".txt": "text/plain; charset=utf-8",
                ".pdf": "application/pdf",
                ".zip": "application/zip",
            }.get(suffix, "application/octet-stream")
            media[path.name] = (path.read_bytes(), mime)
        if not media:
            raise SystemExit(f"{directory} 里没有文件")
        return media

    return {
        "test.png": (make_png(), "image/png"),
        "test.wav": (make_wav(), "audio/wav"),
        "test.txt": (make_text(), "text/plain; charset=utf-8"),
    }


# --------------------------------------------------------------------------
# 组装回复
# --------------------------------------------------------------------------

def pick_files(media: dict[str, tuple[bytes, str]], prefix: str, count: int) -> list[str]:
    """按前缀挑附件名（image→test.png，audio→test.wav，file→test.txt）。"""
    names = [
        name
        for name in media
        if name.startswith(prefix) or (prefix == "image" and name.startswith("test.png"))
    ]
    if not names:
        # 兜底：按扩展名挑
        suffix = {"image": (".png", ".jpg", ".jpeg", ".gif", ".webp"),
                  "audio": (".wav", ".mp3", ".flac", ".ogg"),
                  "file": (".txt", ".pdf", ".zip", ".docx", ".xlsx")}[prefix]
        names = [name for name in media if name.lower().endswith(suffix)]
    if not names:
        return []
    # 数量不够就重复，方便 --count 测"多个附件"
    return [names[i % len(names)] for i in range(max(1, count))]


def attachment(
    kind: str,
    name: str,
    content: bytes,
    mime: str,
    mode: str,
    base_url: str,
) -> dict[str, Any]:
    """把一个附件拼成 OpenAI 兼容的条目。

    * `url`   —— `{"image_url": {"url": "http://.../media/test.png"}}`
    * `b64`   —— `{"image_url": {"url": "data:image/png;base64,..."}}`
    * `mixed` —— 图片用链接，音频/文件用 data URL
    """
    url = f"{base_url}/media/{name}"
    as_data_url = mode == "b64" or (mode == "mixed" and kind != "image")

    if as_data_url:
        encoded = base64.b64encode(content).decode("ascii")
        value = f"data:{mime.split(';')[0]};base64,{encoded}"
    else:
        value = url

    return {f"{kind}_url": {"url": value}}


def build_payload(media: dict[str, tuple[bytes, str]], args: argparse.Namespace,
                  base_url: str) -> dict[str, Any]:
    text = args.content
    message: dict[str, Any] = {"role": "assistant", "content": text}

    for kind, prefix in (("image", "image"), ("audio", "audio"), ("file", "file")):
        names = pick_files(media, prefix, args.count)
        if not names:
            continue
        message[f"{kind}s"] = [
            attachment(kind, name, media[name][0], media[name][1], args.mode, base_url)
            for name in names
        ]

    return message


def openai_response(message: dict[str, Any]) -> dict[str, Any]:
    return {
        "id": "chatcmpl-mock",
        "object": "chat.completion",
        "created": int(time.time()),
        "model": "mock",
        "choices": [
            {
                "index": 0,
                "message": message,
                "finish_reason": "stop",
            }
        ],
        "usage": {
            "prompt_tokens": 12,
            "completion_tokens": 34,
            "total_tokens": 46,
        },
    }


def ollama_response(message: dict[str, Any]) -> dict[str, Any]:
    """Ollama 的 /api/chat 形状（qqpilot5 只读 message.content）。"""
    return {
        "model": "mock",
        "created_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "message": {"role": "assistant", "content": message.get("content", "")},
        "done": True,
        "total_duration": 1,
        "eval_count": 34,
        "prompt_eval_count": 12,
    }


# --------------------------------------------------------------------------
# HTTP 服务
# --------------------------------------------------------------------------

class MockHandler(BaseHTTPRequestHandler):
    server_version = "QQPilotMock/1.0"
    protocol_version = "HTTP/1.1"

    # 由 main() 塞进来
    args: argparse.Namespace
    media: dict[str, tuple[bytes, str]]

    # ---- 小工具 ----------------------------------------------------------

    def log_message(self, fmt: str, *values: Any) -> None:  # 默认的那行太吵
        return

    def _send(self, status: HTTPStatus | int, body: bytes, content_type: str) -> None:
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Connection", "close")
        self.end_headers()
        if self.command != "HEAD":
            self.wfile.write(body)

    def _send_json(self, payload: dict[str, Any]) -> None:
        body = json.dumps(payload, ensure_ascii=False, indent=2).encode("utf-8")
        self._send(HTTPStatus.OK, body, "application/json; charset=utf-8")

    def _base_url(self) -> str:
        host = self.headers.get("Host")
        if not host:
            host = f"{self.args.host}:{self.server.server_address[1]}"
        return f"http://{host}"

    def _log(self, text: str) -> None:
        stamp = time.strftime("%H:%M:%S")
        say(f"[{stamp}] {text}")

    # ---- 路由 ------------------------------------------------------------

    def do_GET(self) -> None:
        path = self.path.split("?", 1)[0]

        if path.startswith("/media/"):
            name = path[len("/media/"):]
            if name not in self.media:
                self._send(HTTPStatus.NOT_FOUND, b"not found", "text/plain; charset=utf-8")
                self._log(f"404 /media/{name}")
                return
            content, mime = self.media[name]
            self._send(HTTPStatus.OK, content, mime)
            self._log(f"200 /media/{name} ({len(content)} 字节, {mime})")
            return

        if path in ("/", "/index.json"):
            payload = {
                "mode": self.args.mode,
                "count": self.args.count,
                "delay": self.args.delay,
                "attachments": {
                    kind: pick_files(self.media, kind, self.args.count)
                    for kind in ("image", "audio", "file")
                },
                "endpoints": {
                    "openai": "POST /v1/chat/completions",
                    "ollama": "POST /api/chat",
                    "media": "GET /media/<名字>",
                },
                "media": sorted(self.media),
                "config.ini": {
                    "modelname": "mock",
                    "server_url": f"{self._base_url()}/v1",
                    "forceollamaapi": "false",
                },
            }
            self._send_json(payload)
            self._log("200 /")
            return

        self._send(HTTPStatus.NOT_FOUND, b"not found", "text/plain; charset=utf-8")
        self._log(f"404 {path}")

    do_HEAD = do_GET

    def do_POST(self) -> None:
        path = self.path.split("?", 1)[0]
        is_ollama = path.rstrip("/").endswith("/api/chat")
        is_openai = path.rstrip("/").endswith("/chat/completions")
        if not (is_ollama or is_openai):
            self._send(HTTPStatus.NOT_FOUND, b"not found", "text/plain; charset=utf-8")
            self._log(f"404 {path}")
            return

        length = int(self.headers.get("Content-Length") or 0)
        raw = self.rfile.read(length) if length else b"{}"
        try:
            request = json.loads(raw.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            self._send(HTTPStatus.BAD_REQUEST, str(error).encode(), "text/plain; charset=utf-8")
            self._log(f"400 {path}: 请求体不是 JSON")
            return

        model = request.get("model", "?")
        messages = request.get("messages") or []
        last = messages[-1] if messages else {}
        preview = str(last.get("content", ""))[:60].replace("\n", " ")
        self._log(f"{path}  模型={model}  消息数={len(messages)}  最后一条: {preview}")

        if self.args.delay > 0:
            time.sleep(self.args.delay)

        message = build_payload(self.media, self.args, self._base_url())
        if is_ollama:
            payload = ollama_response(message)
            detail = "Ollama 形状（只有 content，附件不下发）"
        else:
            payload = openai_response(message)
            counts = {
                kind: len(message.get(f"{kind}s", []))
                for kind in ("image", "audio", "file")
            }
            detail = f"附件 image={counts['image']} audio={counts['audio']} file={counts['file']}"
        print(payload)
        self._send_json(payload)
        self._log(f"200 {path}  模式={self.args.mode}  {detail}")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="QQPilot 测试用的假 LLM 服务（图片 / 音频 / 文件附件）",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=(
            "config.ini 里填：\n"
            "  modelname = mock\n"
            "  server_url = http://127.0.0.1:8000/v1\n"
            "  forceollamaapi = false\n"
        ),
    )
    parser.add_argument("--host", default="127.0.0.1", help="监听地址（默认 127.0.0.1）")
    parser.add_argument("--port", type=int, default=8000, help="监听端口（默认 8000）")
    parser.add_argument(
        "--mode",
        choices=("url", "b64", "mixed"),
        default="mixed",
        help="附件形式：url=全给链接，b64=全给 data URL，mixed=图片给链接、音频/文件给 data URL（默认）",
    )
    parser.add_argument("--count", type=int, default=1, help="每类附件给几个（默认 1）")
    parser.add_argument("--content", default="你好", help="回复正文")
    parser.add_argument("--delay", type=float, default=0.0, help="每次请求延迟多少秒（默认 0）")
    parser.add_argument("--media-dir", default=None, help="用这个目录里的文件当附件，而不是现场生成")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    configure_stdio()
    args = parse_args(argv)
    if args.count < 1:
        raise SystemExit("--count 至少是 1")

    media = build_media(args)
    MockHandler.args = args  # type: ignore[attr-defined]
    MockHandler.media = media  # type: ignore[attr-defined]

    server = ThreadingHTTPServer((args.host, args.port), MockHandler)
    server.daemon_threads = True
    base_url = f"http://{args.host}:{args.port}"

    say("=" * 68)
    say("QQPilot mockServer 已启动")
    say(f"  地址      : {base_url}")
    say(f"  模式      : {args.mode}    每类附件: {args.count}")
    say(f"  附件      : {', '.join(sorted(media))}")
    say(f"  OpenAI    : POST {base_url}/v1/chat/completions")
    say(f"  Ollama    : POST {base_url}/api/chat")
    say(f"  说明页    : GET  {base_url}/")
    say("-" * 68)
    say("config.ini:")
    say("  modelname = mock")
    say(f"  server_url = {base_url}/v1")
    say("  forceollamaapi = false")
    say("-" * 68)
    say("自测（新开一个窗口）:")
    say(
        f'  curl -s -X POST {base_url}/v1/chat/completions '
        '-H "Content-Type: application/json" '
        '-d "{\\"model\\":\\"mock\\",\\"messages\\":[{\\"role\\":\\"user\\",\\"content\\":\\"test\\"}]}"'
    )
    say("=" * 68)

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        say("\n收到 Ctrl+C，正在退出 ...")
    finally:
        server.shutdown()
        server.server_close()
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except OSError as error:
        raise SystemExit(f"启动失败: {error}") from error

