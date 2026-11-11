//! 位图工具：屏幕截图（GDI）、把字节包成 `SoftwareBitmap`、存成 PNG（WinRT 编码器）。
//!
//! 截图走 GDI 而不是 `VisionQQ_C/ScreenCapture.dll`：这个 CLI 不想依赖外部 DLL，
//! 也不需要先把图落盘再读回来 —— 抓到的像素直接在内存里交给 OCR。

use std::path::Path;

use windows::Graphics::Imaging::{
    BitmapAlphaMode, BitmapEncoder, BitmapPixelFormat, SoftwareBitmap,
};
use windows::Storage::Streams::{DataReader, DataWriter, InMemoryRandomAccessStream};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC, SRCCOPY, SelectObject,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

/// 让进程按物理像素工作。
///
/// 不设的话，150% 缩放的屏幕上 `GetSystemMetrics` 和 `BitBlt` 拿到的都是"逻辑像素"，
/// 截出来的图只有实际分辨率的一半多一点，聊天名字那块坐标就对不上了。
pub fn make_dpi_aware() {
    // SAFETY: 只是设置进程的 DPI 感知方式；已经设过会返回错误，忽略即可。
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
}

/// 把一段字节包成 WinRT 的 `IBuffer`。
pub fn buffer_from_bytes(bytes: &[u8]) -> Result<windows::Storage::Streams::IBuffer, String> {
    let writer = DataWriter::new().map_err(|e| format!("创建写入器失败: {e}"))?;
    writer
        .WriteBytes(bytes)
        .map_err(|e| format!("写缓冲区失败: {e}"))?;
    writer
        .DetachBuffer()
        .map_err(|e| format!("取出缓冲区失败: {e}"))
}

/// 截一张全屏图，返回 `Bgra8` 的位图（和 OCR 要求的格式一致）。
pub fn capture_screen() -> Result<SoftwareBitmap, String> {
    let desktop = HWND(std::ptr::null_mut());

    // SAFETY: 所有 GDI 句柄都在本函数末尾释放，失败路径也一样。
    unsafe {
        let screen = GetDC(desktop);
        if screen.is_invalid() {
            return Err("取屏幕 DC 失败".to_string());
        }

        let width = GetSystemMetrics(SM_CXSCREEN);
        let height = GetSystemMetrics(SM_CYSCREEN);
        if width <= 0 || height <= 0 {
            let _ = ReleaseDC(desktop, screen);
            return Err(format!("屏幕尺寸不正常: {width}x{height}"));
        }

        let memory = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, width, height);

        let result = if memory.is_invalid() || bitmap.is_invalid() {
            Err("创建内存位图失败".to_string())
        } else {
            let previous = SelectObject(memory, bitmap);
            let captured = BitBlt(memory, 0, 0, width, height, screen, 0, 0, SRCCOPY)
                .map_err(|e| format!("把屏幕拷进内存位图失败: {e}"))
                .and_then(|()| read_pixels(memory, bitmap, width, height));
            SelectObject(memory, previous);
            captured
        };

        if !bitmap.is_invalid() {
            let _ = DeleteObject(bitmap);
        }
        if !memory.is_invalid() {
            let _ = DeleteDC(memory);
        }
        let _ = ReleaseDC(desktop, screen);

        result
    }
}

/// 从内存位图里读出 BGRA 像素（自上而下），包成 `SoftwareBitmap`。
fn read_pixels(
    memory: windows::Win32::Graphics::Gdi::HDC,
    bitmap: windows::Win32::Graphics::Gdi::HBITMAP,
    width: i32,
    height: i32,
) -> Result<SoftwareBitmap, String> {
    let mut info = BITMAPINFO::default();
    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = width;
    // 负高度 = 自上而下，省掉一次整图翻转。
    info.bmiHeader.biHeight = -height;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB.0;

    let mut pixels = vec![0u8; width as usize * height as usize * 4];
    // SAFETY: pixels 足够大（width*height*4），info 是本函数栈上的有效结构。
    let lines = unsafe {
        GetDIBits(
            memory,
            bitmap,
            0,
            height as u32,
            Some(pixels.as_mut_ptr().cast()),
            &mut info,
            DIB_RGB_COLORS,
        )
    };
    if lines != height {
        return Err(format!("读屏幕像素只拿到 {lines}/{height} 行"));
    }

    let buffer = buffer_from_bytes(&pixels)?;
    SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
        &buffer,
        BitmapPixelFormat::Bgra8,
        width,
        height,
        BitmapAlphaMode::Premultiplied,
    )
    .map_err(|e| format!("构造截图位图失败: {e}"))
}

/// 把位图存成 PNG。
pub fn save_png(path: &Path, bitmap: &SoftwareBitmap) -> Result<(), String> {
    let stream = InMemoryRandomAccessStream::new().map_err(|e| format!("创建内存流失败: {e}"))?;
    let encoder_id =
        BitmapEncoder::PngEncoderId().map_err(|e| format!("取 PNG 编码器失败: {e}"))?;
    let encoder = BitmapEncoder::CreateAsync(encoder_id, &stream)
        .map_err(|e| format!("创建编码器失败: {e}"))?
        .get()
        .map_err(|e| format!("创建编码器失败: {e}"))?;
    encoder
        .SetSoftwareBitmap(bitmap)
        .map_err(|e| format!("塞入位图失败: {e}"))?;
    encoder
        .FlushAsync()
        .map_err(|e| format!("编码 PNG 失败: {e}"))?
        .get()
        .map_err(|e| format!("编码 PNG 失败: {e}"))?;

    let size = stream
        .Size()
        .map_err(|e| format!("读内存流长度失败: {e}"))?;
    stream.Seek(0).map_err(|e| format!("回到流开头失败: {e}"))?;
    let reader =
        DataReader::CreateDataReader(&stream).map_err(|e| format!("创建读取器失败: {e}"))?;
    reader
        .LoadAsync(size as u32)
        .map_err(|e| format!("加载 PNG 数据失败: {e}"))?
        .get()
        .map_err(|e| format!("加载 PNG 数据失败: {e}"))?;

    let mut bytes = vec![0u8; size as usize];
    reader
        .ReadBytes(&mut bytes)
        .map_err(|e| format!("拷贝 PNG 数据失败: {e}"))?;
    std::fs::write(path, bytes).map_err(|e| format!("写 {} 失败: {e}", path.display()))
}
