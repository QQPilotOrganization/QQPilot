# lr

用 **Windows 自带的 OCR**（`Windows.Media.Ocr`）读 QQ 截图里的文字。纯 Rust CLI，
不依赖任何第三方 OCR 库，也不需要联网。

## 用法

```powershell
# 不带路径：先截一张全屏图（默认存成 ./screenshot.png），再读"聊天名字"那块
lr

#获取群聊名称到txt中
lr>o.txt -l zh-Hans-CN

# 目录：把目录里所有图片整张识别
lr screenshots\

# 单张图片
lr a.png
```

```powershell
lr -s                    # 系统装了哪些 OCR 语言
lr -j                    # 详细 JSON（每行、每个字的坐标）
lr -o name.json -j       # 写到文件
lr -l zh-Hans-CN         # 指定语言（默认跟随系统）
lr -c other.ini          # 指定 config.ini
lr --save shot.png       # 截图存到别处
lr --no-save             # 只识别，不落盘
lr --image shot.png      # 不截图，直接读现成的图（调试用）
lr --bbox 376,86,750,32  # 手动指定裁剪区域（调参用）
lr --upscale 2           # 手动指定放大倍数（1..8，调参用）
```

退出码：`0` 成功，`1` 参数错误，`2` 读配置/文件失败，`3` 有图片识别失败。

## 截图是怎么做的

不带路径时 `lr` 自己截屏，**不依赖 `VisionQQ_C/ScreenCapture.dll`，也不需要外部截图工具**：

1. `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` —— 不然高分屏上拿到的是逻辑像素；
2. GDI `BitBlt` 屏幕 → 内存位图 → `GetDIBits`（负高度 = 自上而下）；
3. 像素直接包成 `SoftwareBitmap` 交给 OCR；
4. 需要的话再用 WinRT 的 `BitmapEncoder` 存成 PNG（默认 `screenshot.png`，`--no-save` 关掉）。

## 坐标怎么来的

`config.ini` 的 `[general]` 里读 `width` / `height` / `scale`（读不到就用 1285×720 / 1.0）。
和 qqpilot5 同一套口径：

```text
实际像素 = 相对坐标 × (width × scale, height × scale)
```


## 两个踩过的坑（都写在代码注释里）

1. **文字贴边会让 OCR 完全失效**：同样一段字，裁成 750×32 一个字都读不出，
   上下各留 16 像素（750×64）就能全读对。所以裁剪时按高度的 1/2 加了留白
   （`pad_region`），坐标仍然换算回原图；留白带进来的"邻居文字"会按请求区域
   再过滤一遍（`intersects`），只留落在聊天名字那条里的行。
2. **`BitmapDecoder.GetSoftwareBitmapTransformedAsync` 不能用**：`Bounds` + 缩放的
   组合在这台机器上要么返回空白图、要么报 `E_INVALIDARG`，所以改成自己按像素
   裁剪 + 双线性放大（`crop_and_scale`）。

## 和 qcocr 的关系

实现只参考了 `Windows.Media.Ocr` 的公开接口（解码 → `OcrEngine.RecognizeAsync` →
`Lines` / `Words` / `BoundingRect`），**没有使用 `C:\Users\Develop\qcocr` 的任何代码**——
那个项目没有开源许可，不能拷。这里的裁剪、留白、坐标换算、语言处理、CLI
都是另写的。

## 测试

```powershell
cargo test
```

测试已通过。
