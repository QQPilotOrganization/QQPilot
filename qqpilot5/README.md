# QQPilot5

`QQPilot4`（C# / .NET 10）的 Rust 移植版。行为与 C# 版保持一致，仍然复用 VisionQQ_C
编译出来的原生 DLL，不重写视觉与输入引擎。

## 构建

```powershell
cargo build --release
```

产出 `target\release\qqpilot5.exe`。

## 运行前需要准备的文件

qqpilot5.exe 必须在**同时包含下面这些文件**的目录里运行（当前工作目录就是它读取
配置和读写 `screenshot.png` / `log.txt` / `tokencount.txt` 的地方）：

| 类别 | 文件 | 来源 |
| --- | --- | --- |
| 原生 DLL | `InputEvent.dll` `Vision.dll` `ScreenCapture.dll` `uploadFile.dll` `FocusQQWindow2.dll` | `QQPilot4\bin\Release\net10.0\`，或 VisionQQ_C 各工程编译产物 |
| 辅助程序 | `ScaleToINI.exe`（启动时刷新 `scale`）、`uploadImage2.exe`（模板匹配失败时兜底） | 同上 |
| 配置 | `config.ini` `system.txt` `extra.json` | 仓库根目录 / `QQPilot4\bin\...` |
| 模板图 | `copy.png` `uploadImage.png` | `QQPilot4\bin\...` |
| 内置模型 | `datasetTiny.json`（`server_url = builtin` 时必需） | 同上 |
| 图片素材 | `Images\` 目录（随机发图时用） | 同上 |

运行时还会自动生成 `screenshot.png` / `log.txt` / `tokencount.txt` / `temp\`。

## 配置文件（已合并解析）

C# 版把 `config.ini` 读了四次、`extra.json` 和 `system.txt` 各读一次，散落在
`Program` / `GUIOperation` / `Vision` / `Answer` 四个类里。Rust 版统一到
**`src/config.rs`** 一处：进程启动时读一次，之后全流程通过 `config::get()` 取。

要新增一个配置项，只改 `src/config.rs` 三处：

1. 在 `struct Config` 里加字段；
2. 在 `impl Default for Config` 里写出厂默认值；
3. 在 `Config::load` 里加一行 `get_str` / `get_int` / `get_num` / `get_bool`。

缺失的键会回落到默认值并打一条 `[WARN]`，不会再像 C# 版那样直接抛异常退出。

## 与 C# 版的差异

见仓库根目录的迁移说明；主要差异集中在 `src/config.rs`（合并解析）与
`src/native.rs`（`libloading` 取代 `DllImport`）。

## 测试

```powershell
cargo test
```

覆盖配置解析、聊天记录解析（含换行/图片路径/自身消息判定）、坐标换算与 FFI ABI。
把五个原生 DLL 放到本目录后，`native::tests::every_export_resolves_against_real_dlls`
还会实际校验全部导出函数名与结构体返回约定；没有 DLL 时该用例自动跳过。

```powershell
# 手动验证剪贴板存取（会覆盖系统剪贴板）
cargo test clipboard -- --ignored --nocapture
```
