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

写回 `config.ini` 时会先把整份文件读进来再改界面管的那 18 个键，
所以 `scale` / `nt_data` / `system` 等界面不管的键、以及文件里原有的键顺序都会保留下来。

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
