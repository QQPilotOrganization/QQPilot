# option3

QQPilot 设置界面（`QQPilotGUISharp` / `Option2.csproj` 的 Rust 移植版）。

用 [native-windows-gui](https://crates.io/crates/native-windows-gui) 直接包 Win32 原生控件，
控件种类、坐标、尺寸全部照搬 C# 的 `Form1.Designer.cs`，行为照搬 `Form1.cs`。

## 构建 / 运行

```powershell
cargo build --release
```

```powershell
# 在放着 config.ini 的目录里运行（程序按当前工作目录读写配置）
.\target\release\option3.exe
```

图标已用 `include_bytes!` 打进可执行文件，不需要额外带 `option.ico`。

## 读写哪些文件

| 文件 | 何时读 | 何时写 |
| --- | --- | --- |
| `config.ini` | 启动时填充界面 | 点「保存设置」或关闭窗口 |
| `system.txt` | 启动时填进「提示文本」 | 同上 |
| `tokencount.txt` | 显示在「Token用量」 | 点「重置计数器」，或文件读不到时清零 |
| `extra.json` | 不读，只交给记事本编辑 | — |

写回 `config.ini` 时会先把整份文件读进来再改界面管的那 19 个键，
所以 `scale` / `nt_data` / `system` 等界面不管的键、以及文件里原有的键顺序都会保留下来。

## 文案翻译（localization.json）

界面上的每一条文案都从 **`localization.json`** 取，代码里不写死中文。
**这个文件必须和 exe 放在一起**，缺了的话所有文案会显示成 `X<key>` 并把缺的 key 打到 stderr 上
（`localization::get` 的兜底行为，方便一眼看出漏了哪条）。

`src/localization.rs` 提供三个入口：

| 函数 | 用途 |
| --- | --- |
| `load()` | 读 `localization.json`，返回一个 `Translation` |
| `get(&Translation, key)` | 取一条翻译，用于运行期拼接（`format!`） |
| `text(key) -> &'static str` | 取一条翻译并固化成立即用的 `&'static str`，给 `#[nwg_control(text: ...)]` 用 |

`text()` 的返回值只对每个 key 泄漏一次（缓存过），key 的数量等于控件数，不会随运行增长。

翻译表的**唯一来源是 `localization.rs` 里 `default_cfg` 测试**中的那张表：

```powershell
cargo test localization::tests::default_cfg   # 重新生成 localization.json
```

新增文案的流程：改 `default_cfg` → 跑一次上面的测试生成 JSON → 在代码里用 `get`/`text` 引用。
有一条测试会把两者对照（引用不到 / 定义没用都会显出来）。

> 键名里唯一没走翻译的是服务器下拉的第一项 `"ollama"` —— 它是写回 `config.ini` 的协议值，
> 不是给人看的文案。

## 控件对应关系

AntdUI 的控件在 Win32 里没有完全对等的东西，映射如下：

| C# (AntdUI) | Rust (native-windows-gui) | 说明 |
| --- | --- | --- |
| `Input` | `TextInput` | 多行的用 `TextBox` |
| `InputNumber` | `TextInput` + `ES_NUMBER` | 上下限在读写时夹紧 |
| `Switch` | `CheckBox` | 去掉了默认标题文字 |
| `Select` | `ComboBox` | 高 36 由 `CB_SETITEMHEIGHT` 对齐 |
| `Slider` | `TrackBar` | 数值回显在左侧标签里 |
| `ButtonShadow` | `Button` | |
| `Label` | `Label` | |
| `Tooltip` | `Label` | NWG 没有常显的可点击提示控件 |

## 与 C# 版的差异

1. **下拉框选中项的可视文本**：C# 先设置 `SelectedIndex`（会触发联动）再用配置原文覆盖输入框。
   NWG 的 `CB_SETCURSEL` 不会触发联动，所以这里直接按同样的顺序显式赋值，最终状态一致。
2. **「请求的额外参数」**：C# 用 `WaitForExit()` 等记事本关闭，会把设置窗口一起卡住；
   这里改成不等待，窗口保持可用。
3. **DPI**：开启了 `high-dpi` 特性并调用 `SetProcessDPIAware()`，与 C# 版 WinForms 默认的
   `SystemAware` 一致 —— 界面按 DPI 放大而不是被系统拉伸成模糊图。
4. **保存提示**：保存成功不弹窗（与 C# 一致），失败时弹错误框（C# 会直接抛异常）。

## 测试

```powershell
cargo test
```

覆盖配置读写（含"写回后不丢键、不打乱顺序"）与服务地址到下拉框下标的映射。
