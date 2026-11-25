# mockServer

QQPilot 的假 LLM 服务，用来测"模型回复里带**图片 / 音频 / 文件**"这条链路。
纯 Python 标准库（3.8+），不用装包、不联网。

```
mockServer/
  mockServer.py   服务本体
  selftest.py     自检（起服务、跑一遍所有链路、校验 png/wav 合不合法）
```

## 启动

```powershell
python mockServer.py                       # 127.0.0.1:8000，mixed 模式
python mockServer.py --port 9000           # 换端口
python mockServer.py --mode b64            # 附件全用 data URL
python mockServer.py --count 2             # 每类两个附件
python mockServer.py --delay 1.5           # 每次请求慢 1.5 秒
python mockServer.py --media-dir .\media   # 用自己准备的 png/wav/txt 当附件
```

启动后它会把要填的 `config.ini` 和一条 `curl` 自测命令直接打出来。

## 参数

| 参数 | 默认 | 说明 |
|---|---|---|
| `--host` | `127.0.0.1` | 监听地址 |
| `--port` | `8000` | 监听端口 |
| `--mode` | `mixed` | `url` / `b64` / `mixed`，见下表 |
| `--count` | `1` | 每类给几个附件（不够就重复用同一个文件） |
| `--content` | 一句测试文本 | 回复正文 |
| `--delay` | `0` | 每次请求延迟秒数，用来模拟慢服务 |
| `--media-dir` | 无 | 用这个目录里的文件当附件，而不是现场生成 |

三种模式对应的客户端行为：

| `--mode` | 图片 | 音频 | 文件 | 测什么 |
|---|---|---|---|---|
| `url` | `http://` 链接 | 链接 | 链接 | qqpilot5 的**下载**（要求 `remote_images/audios/files = true`） |
| `b64` | `data:` URL | data URL | data URL | 纯本地解码，不碰网络 |
| `mixed` | 链接 | data URL | data URL | 一次把两条路都走一遍（默认） |

把对应开关关掉（`remote_*=false`）时，`url` 模式下 qqpilot5 会打印
"对应的 remote_\* 开关没开，已跳过远程链接"，正好用来验证开关。

## 接口

| 方法 | 路径 | 说明 |
|---|---|---|
| POST | `/v1/chat/completions` | OpenAI 兼容回复（qqpilot5 默认） |
| POST | `/api/chat` | Ollama 形状（`forceollamaapi=true` 时） |
| GET | `/media/test.png` 等 | 附件本体（`Content-Type` 正确，可用来单独测下载） |
| GET | `/` | 说明页：当前模式、附件映射、要填的配置 |

回复里附件的形状（`kind` = `image` / `audio` / `file`）：

```json
{ "image_url": { "url": "http://127.0.0.1:8000/media/test.png" } }
{ "audio_url": { "url": "data:audio/wav;base64,..." } }
{ "file_url":  { "url": "data:text/plain;base64,..." } }
```

生成的文件都很小（48x48 PNG、0.3 秒 22.05kHz WAV、一小段 txt），
所以就算走 base64 也不会把日志刷爆。

## 接到 qqpilot5 上

把 `releaseBuild\1.5.21 x64\data\config.ini` 里这几项改成：

```ini
modelname = mock
server_url = http://127.0.0.1:8000/v1
forceollamaapi = false
```

然后跑 `qqpilot5.exe`：它每轮取到的新消息都会发到这个假服务，
回复里带的图片/音频/文件应该依次被上传到 QQ 聊天窗口（图片 → 文件 → 音频）。

> `server_url` 不要带 `/chat/completions`，客户端会自己拼。
> 用 Ollama 形状的话把 `forceollamaapi` 设成 `true`，`server_url` 填 `http://127.0.0.1:8000`
> （它只走 `/api/chat`，而且不会下发附件）。

## 自检

```powershell
python selftest.py            # 76 项检查，覆盖三种模式 + Ollama + 媒体文件合法性
python selftest.py --quick    # 只跑 mixed
```

它会在随机端口起一个临时实例，检查：

* `GET /` 说明页的字段
* 三种模式下 `images/audios/files` 的字段形状（`{kind}_url.url`）
* 每个附件该是 `http://` 还是 `data:`（跟模式一致）
* `--count 2` 真的给了两个
* `--delay 0.5` 真的慢了
* `/media/test.png` 的每个 PNG chunk CRC 都对、尺寸 48x48
* `/media/test.wav` 是合法 RIFF/WAVE、22.05kHz/16bit

## 自己准备附件

```powershell
mkdir media
copy 你的图片.png media\image1.png
copy 你的录音.wav media\audio1.wav
python mockServer.py --media-dir .\media
```

`--media-dir` 模式下不会生成 test.png/test.wav，而是按扩展名从目录里挑
（`image*` / `audio*` / `file*` 前缀优先，否则按 `.png` `.wav` `.txt` 这类后缀）。
想测"音频能不能真的放出来"，就放一个能听出来的 wav 进去。

> 注意 `--mode b64` 会把整个文件读进内存再 base64：目录里有几十 MB 的 wav 时，
> 响应会非常大（比如 21MB 的 wav → 约 28MB 的 JSON）。这种时候用 `--mode url`，
> 让客户端去 `/media/<名字>` 下载。
