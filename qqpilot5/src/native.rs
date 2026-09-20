//! 原生 DLL 绑定层。
//!
//! QQPilot4 (C#) 用 `[DllImport]` 直接绑定了 VisionQQ_C 编译出来的五个 DLL；
//! 这里用 `libloading` 在运行时按相同顺序（先 exe 目录、再当前目录）加载，
//! 保证行为与 C# 版一致。
//!
//! 导出签名是从 C 源码逐个核对过的：
//!   * `InputEvent/dllmain.cpp` + `mouseEvent.cpp` + `keybdevent.cpp`
//!   * `Vision/dllmain.h` + `Vision/dllmain.cpp`
//!   * `ScreenCapture/dllmain.cpp`
//!   * `uploadFile/dllmain.cpp`
//!   * `FocusQQWindow2/dllmain.cpp`

use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

use crate::localization;
use crate::log;

/// `Vision/dllmain.h` 的 `typedef struct Point { unsigned x, y; }`。
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

/// Win32 `RECT`（C 侧 `rect()` 实际写入的是 x / y / width / height）。
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

// --- 加载辅助 ---

/// 与 C# 版相同：先在可执行文件目录、再在当前工作目录找 DLL。
fn find_dll(name: &str) -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
    {
        candidates.push(dir.join(name));
    }
    if let Ok(dir) = std::env::current_dir() {
        candidates.push(dir.join(name));
    }
    for candidate in &candidates {
        if candidate.is_file() {
            return Ok(candidate.clone());
        }
    }
    let translate = localization::load();
    Err(format!(
        "{}: {name}",
        localization::get(&translate, "error.dll.notfound")
    ))
}

fn open_library(name: &str) -> Result<Library, String> {
    let translate = localization::load();
    let path = find_dll(name)?;
    // SAFETY: 只是加载一个动态库；DLL 自身的初始化由 DllMain 负责。
    unsafe { Library::new(&path) }.map_err(|e| {
        format!(
            "{} {}: {e}",
            localization::get(&translate, "error.dll.loadfailed"),
            path.display()
        )
    })
}

/// 取一个导出函数的地址并拷贝成普通函数指针。
///
/// # Safety
/// 调用方必须保证 `T` 与 DLL 中该符号的真实签名完全一致。
unsafe fn load_fn<T: Copy>(lib: &Library, dll: &str, name: &str) -> Result<T, String> {
    let translate = localization::load();
    let mut symbol_name = name.as_bytes().to_vec();
    symbol_name.push(0);
    // SAFETY: 由调用方保证签名匹配；symbol_name 已按 C 字符串以 NUL 结尾。
    let symbol: Symbol<T> = unsafe { lib.get(&symbol_name) }.map_err(|e| {
        format!(
            "{} {dll} -> {name}: {e}",
            localization::get(&translate, "error.dll.symbolmissing")
        )
    })?;
    Ok(*symbol)
}

// --- InputEvent.dll ---

type SmoothMouseGotoFn = unsafe extern "C" fn(u32, u32) -> bool;
type LClickFn = unsafe extern "C" fn(u32, u32) -> bool;
type ScrollFn = unsafe extern "C" fn(i32) -> bool;
type GetVkKeyFn = unsafe extern "C" fn(*const std::ffi::c_char) -> u16;
type HotKeyFn = unsafe extern "C" fn(u16, u16) -> bool;
type PressFn = unsafe extern "C" fn(u16) -> bool;
type SimpleActionFn = unsafe extern "C" fn() -> bool;
type DpiAwarenessFn = unsafe extern "C" fn() -> bool;

/// `InputEvent.dll`：鼠标与键盘注入。
pub struct InputEvent {
    smooth_mouse_goto: SmoothMouseGotoFn,
    l_click: LClickFn,
    scroll_down: ScrollFn,
    get_vk_key: GetVkKeyFn,
    hot_key: HotKeyFn,
    press: PressFn,
    mouse_down: SimpleActionFn,
    mouse_up: SimpleActionFn,
    dpi_awareness: DpiAwarenessFn,
    /// 必须最后析构：上面的函数指针都指向这块库的代码段。
    _lib: Library,
}

impl InputEvent {
    fn load() -> Result<Self, String> {
        let lib = open_library("InputEvent.dll")?;
        // SAFETY: 每个符号的签名都与 InputEvent.dll 的 C 导出逐一核对过。
        unsafe {
            Ok(Self {
                smooth_mouse_goto: load_fn(&lib, "InputEvent.dll", "SmoothMousegoto")?,
                l_click: load_fn(&lib, "InputEvent.dll", "Lclick")?,
                scroll_down: load_fn(&lib, "InputEvent.dll", "scrollDown")?,
                get_vk_key: load_fn(&lib, "InputEvent.dll", "getVkKey")?,
                hot_key: load_fn(&lib, "InputEvent.dll", "hotKey")?,
                press: load_fn(&lib, "InputEvent.dll", "press")?,
                mouse_down: load_fn(&lib, "InputEvent.dll", "LmouseDown")?,
                mouse_up: load_fn(&lib, "InputEvent.dll", "LmouseUp")?,
                dpi_awareness: load_fn(&lib, "InputEvent.dll", "DPIAwarenessPrologue")?,
                _lib: lib,
            })
        }
    }

    /// 对应 `GUIOperation.SmoothMousegoto`。
    pub fn smooth_mouse_goto(&self, x: u32, y: u32) -> bool {
        // SAFETY: 函数指针在构造时已解析，签名与 C 侧一致。
        unsafe { (self.smooth_mouse_goto)(x, y) }
    }

    /// 对应 `GUIOperation.Lclick`。
    pub fn l_click(&self, x: u32, y: u32) -> bool {
        // SAFETY: 同上。
        unsafe { (self.l_click)(x, y) }
    }

    /// 对应 `GUIOperation.ScrollDown`。
    pub fn scroll_down(&self, delta: i32) -> bool {
        // SAFETY: 同上。
        unsafe { (self.scroll_down)(delta) }
    }

    /// 对应 `GUIOperation.getVkKey`。
    pub fn get_vk_key(&self, key: &str) -> u16 {
        let Ok(name) = std::ffi::CString::new(key) else {
            return 0;
        };
        // SAFETY: name 是以 NUL 结尾的 C 字符串，生命周期覆盖本次调用。
        unsafe { (self.get_vk_key)(name.as_ptr()) }
    }

    /// 对应 `GUIOperation.press`。
    pub fn press(&self, vk: u16) -> bool {
        // SAFETY: 同上。
        unsafe { (self.press)(vk) }
    }

    /// 对应 `GUIOperation.hotKey`。
    pub fn hot_key(&self, modifier: u16, key: u16) -> bool {
        // SAFETY: 同上。
        unsafe { (self.hot_key)(modifier, key) }
    }

    /// 对应 `GUIOperation.LmouseDown`。
    pub fn mouse_down(&self) -> bool {
        // SAFETY: 同上。
        unsafe { (self.mouse_down)() }
    }

    /// 对应 `GUIOperation.LmouseUp`。
    pub fn mouse_up(&self) -> bool {
        // SAFETY: 同上。
        unsafe { (self.mouse_up)() }
    }

    /// 对应 `GUIOperation.Init` 里的 DPI 感知设置。
    pub fn dpi_awareness_prologue(&self) -> bool {
        // SAFETY: 同上。
        unsafe { (self.dpi_awareness)() }
    }
}

// --- Vision.dll ---

type ContainsRedDotFn = unsafe extern "C" fn(Rect, *const std::ffi::c_char) -> Point;
type ContainsBlueFn = unsafe extern "C" fn(*const std::ffi::c_char) -> Point;
type RectFn = unsafe extern "C" fn(u32, u32, u32, u32) -> Rect;
type MatchBeginFn =
    unsafe extern "C" fn(*const std::ffi::c_char, *const std::ffi::c_char, i32, i32) -> i32;
type MatchBeginScaledFn =
    unsafe extern "C" fn(*const std::ffi::c_char, *const std::ffi::c_char, i32, i32, f32) -> i32;
type MatchNextFn = unsafe extern "C" fn(i32) -> Point;
type MatchEndFn = unsafe extern "C" fn();

/// `Vision.dll`：红点检测与多尺度模板匹配。
pub struct VisionDll {
    contains_red_dot: ContainsRedDotFn,
    contains_blue: ContainsBlueFn,
    rect: RectFn,
    match_multi_scale_begin: MatchBeginFn,
    match_selected_scale_begin: MatchBeginScaledFn,
    match_next: MatchNextFn,
    match_end: MatchEndFn,
    _lib: Library,
}

impl VisionDll {
    fn load() -> Result<Self, String> {
        let lib = open_library("Vision.dll")?;
        // SAFETY: 每个符号的签名都与 Vision.dll 的 C 导出逐一核对过。
        unsafe {
            Ok(Self {
                contains_red_dot: load_fn(&lib, "Vision.dll", "containsRedDot")?,
                contains_blue: load_fn(&lib, "Vision.dll", "containsBlue")?,
                rect: load_fn(&lib, "Vision.dll", "rect")?,
                match_multi_scale_begin: load_fn(
                    &lib,
                    "Vision.dll",
                    "matchTemplatesMultiScaleBegin",
                )?,
                match_selected_scale_begin: load_fn(
                    &lib,
                    "Vision.dll",
                    "matchTemplatesSelectedScaleBegin",
                )?,
                match_next: load_fn(&lib, "Vision.dll", "matchTemplateNext")?,
                match_end: load_fn(&lib, "Vision.dll", "matchTemplateEnd")?,
                _lib: lib,
            })
        }
    }

    /// 对应 `NativeMethods.containsRedDot(rect, path)`。
    pub fn contains_red_dot(&self, rect: Rect, image_path: &str) -> Point {
        let Ok(path) = std::ffi::CString::new(image_path) else {
            return Point::default();
        };
        // SAFETY: path 是以 NUL 结尾的 C 字符串，生命周期覆盖本次调用。
        unsafe { (self.contains_red_dot)(rect, path.as_ptr()) }
    }

    /// 对应 `NativeMethods.containsBlue(path)`。
    pub fn contains_blue(&self, image_path: &str) -> Point {
        let Ok(path) = std::ffi::CString::new(image_path) else {
            return Point::default();
        };
        // SAFETY: 同上。
        unsafe { (self.contains_blue)(path.as_ptr()) }
    }

    /// 对应 `NativeMethods.rect`。
    pub fn rect(&self, x: u32, y: u32, width: u32, height: u32) -> Rect {
        // SAFETY: 纯值构造，无指针参数。
        unsafe { (self.rect)(x, y, width, height) }
    }

    /// 对应 `NativeMethods.matchTemplatesMultiScaleBegin`。
    pub fn match_multi_scale_begin(
        &self,
        image_path: &str,
        template_path: &str,
        tolerance: i32,
        max_count: i32,
    ) -> i32 {
        let (Ok(image), Ok(template)) = (
            std::ffi::CString::new(image_path),
            std::ffi::CString::new(template_path),
        ) else {
            return -2;
        };
        // SAFETY: 两个 C 字符串的生命周期覆盖本次调用。
        unsafe {
            (self.match_multi_scale_begin)(image.as_ptr(), template.as_ptr(), tolerance, max_count)
        }
    }

    /// 对应 `NativeMethods.matchTemplatesSelectedScaleBegin`。
    pub fn match_selected_scale_begin(
        &self,
        image_path: &str,
        template_path: &str,
        tolerance: i32,
        max_count: i32,
        scale: f32,
    ) -> i32 {
        let (Ok(image), Ok(template)) = (
            std::ffi::CString::new(image_path),
            std::ffi::CString::new(template_path),
        ) else {
            return -2;
        };
        // SAFETY: 同上。
        unsafe {
            (self.match_selected_scale_begin)(
                image.as_ptr(),
                template.as_ptr(),
                tolerance,
                max_count,
                scale,
            )
        }
    }

    /// 对应 `NativeMethods.matchTemplateNext`。
    pub fn match_next(&self, index: i32) -> Point {
        // SAFETY: 纯值调用；索引由调用方按 DLL 返回的数量约束。
        unsafe { (self.match_next)(index) }
    }

    /// 对应 `NativeMethods.matchTemplateEnd`。
    pub fn match_end(&self) {
        // SAFETY: 释放 DLL 内部匹配上下文，与 Begin 成对调用。
        unsafe { (self.match_end)() }
    }
}

// --- ScreenCapture.dll ---

type ScreenshotFn = unsafe extern "C" fn(i32, i32, i32, i32) -> i32;
type FullScreenshotFn = unsafe extern "C" fn() -> i32;

/// `ScreenCapture.dll`：抓屏到 `screenshot.png`。
pub struct ScreenCapture {
    screenshot: ScreenshotFn,
    full_screenshot: FullScreenshotFn,
    _lib: Library,
}

impl ScreenCapture {
    fn load() -> Result<Self, String> {
        let lib = open_library("ScreenCapture.dll")?;
        // SAFETY: 签名与 ScreenCapture/dllmain.cpp 的导出一致。
        unsafe {
            Ok(Self {
                screenshot: load_fn(&lib, "ScreenCapture.dll", "screenshot")?,
                full_screenshot: load_fn(&lib, "ScreenCapture.dll", "fullScreenshot")?,
                _lib: lib,
            })
        }
    }

    /// 对应 `NativeMethods.screenshot`。返回 1 表示失败。
    pub fn screenshot(&self, x: i32, y: i32, width: i32, height: i32) -> i32 {
        // SAFETY: 纯值调用。
        unsafe { (self.screenshot)(x, y, width, height) }
    }

    /// 对应 `NativeMethods.fullScreenshot`。返回 1 表示失败。
    pub fn full_screenshot(&self) -> i32 {
        // SAFETY: 纯值调用。
        unsafe { (self.full_screenshot)() }
    }
}

// --- uploadFile.dll ---

type UploadFn = unsafe extern "C" fn() -> i32;
type EscapeFn = unsafe extern "C" fn();
type UploadSelectedImageFn = unsafe extern "C" fn(*const u16) -> i32;

/// `uploadFile.dll`：在"请选择"文件对话框里填路径并确认。
pub struct UploadFile {
    upload: UploadFn,
    escape: EscapeFn,
    upload_selected_image: UploadSelectedImageFn,
    _lib: Library,
}

impl UploadFile {
    fn load() -> Result<Self, String> {
        let lib = open_library("uploadFile.dll")?;
        // SAFETY: 签名与 uploadFile/dllmain.cpp 的导出一致。
        unsafe {
            Ok(Self {
                upload: load_fn(&lib, "uploadFile.dll", "upload")?,
                escape: load_fn(&lib, "uploadFile.dll", "escape")?,
                upload_selected_image: load_fn(&lib, "uploadFile.dll", "uploadSelectedImage")?,
                _lib: lib,
            })
        }
    }

    /// 对应 `Upload.upload()`：从 `./Images` 随机挑一张图上传。
    pub fn upload(&self) -> i32 {
        // SAFETY: 纯值调用。
        unsafe { (self.upload)() }
    }

    /// 对应 `Upload.escape()`：关掉文件选择对话框。
    pub fn escape(&self) {
        // SAFETY: 纯值调用。
        unsafe { (self.escape)() }
    }

    /// 对应 `GUIOperation.UploadSelectedFile(file)`。
    ///
    /// C 侧签名是 `wchar_t*`，所以要传 UTF-16 且以 NUL 结尾。
    pub fn upload_selected_image(&self, file: &str) -> i32 {
        let mut wide: Vec<u16> = file.encode_utf16().collect();
        wide.push(0);
        // SAFETY: wide 以 0 结尾，生命周期覆盖本次调用。
        unsafe { (self.upload_selected_image)(wide.as_ptr()) }
    }
}

// --- FocusQQWindow2.dll ---

type FocusFn = unsafe extern "C" fn(bool) -> i32;

/// `FocusQQWindow2.dll`：把 QQ 窗口置顶/取消置顶并设成目标尺寸。
pub struct FocusQqWindow {
    focus: FocusFn,
    _lib: Library,
}

impl FocusQqWindow {
    fn load() -> Result<Self, String> {
        let lib = open_library("FocusQQWindow2.dll")?;
        // SAFETY: `focus` 的 C 侧签名是 `int focus(bool)`。
        unsafe {
            Ok(Self {
                focus: load_fn(&lib, "FocusQQWindow2.dll", "focus")?,
                _lib: lib,
            })
        }
    }

    /// 对应 `Focus.focus(flag)`。
    pub fn focus(&self, always_on_top: bool) -> i32 {
        // SAFETY: 纯值调用。
        unsafe { (self.focus)(always_on_top) }
    }
}

// --- 全局实例 ---

/// 加载失败时直接终止进程，与 C# 版抛未捕获异常的可见结果一致，但错误信息更明确。
fn load_or_exit<T>(result: Result<T, String>) -> T {
    match result {
        Ok(value) => value,
        Err(message) => {
            log::error(message);
            std::process::exit(1);
        }
    }
}

macro_rules! lazy_library {
    ($name:ident, $ty:ty, $loader:expr, $doc:literal) => {
        #[doc = $doc]
        pub fn $name() -> &'static $ty {
            static INSTANCE: OnceLock<$ty> = OnceLock::new();
            INSTANCE.get_or_init(|| load_or_exit($loader))
        }
    };
}

lazy_library!(
    input_event,
    InputEvent,
    InputEvent::load(),
    "全局 `InputEvent.dll` 实例。"
);
lazy_library!(
    vision_dll,
    VisionDll,
    VisionDll::load(),
    "全局 `Vision.dll` 实例。"
);
lazy_library!(
    screen_capture,
    ScreenCapture,
    ScreenCapture::load(),
    "全局 `ScreenCapture.dll` 实例。"
);
lazy_library!(
    upload_file,
    UploadFile,
    UploadFile::load(),
    "全局 `uploadFile.dll` 实例。"
);
lazy_library!(
    focus_qq_window,
    FocusQqWindow,
    FocusQqWindow::load(),
    "全局 `FocusQQWindow2.dll` 实例。"
);

#[cfg(test)]
mod tests {
    use super::*;

    /// 逐一解析五个 DLL 的全部导出函数。
    ///
    /// 找不到 DLL 时自动跳过（例如在没拷贝原生依赖的开发机上跑 `cargo test`），
    /// 把 DLL 放到 `qqpilot5/` 或可执行文件旁边即可真正生效。
    #[test]
    fn every_export_resolves_against_real_dlls() {
        if find_dll("Vision.dll").is_err() {
            eprintln!("跳过：当前目录没有原生 DLL，无法校验导出函数");
            return;
        }

        InputEvent::load().expect("InputEvent.dll 导出函数缺失");
        VisionDll::load().expect("Vision.dll 导出函数缺失");
        ScreenCapture::load().expect("ScreenCapture.dll 导出函数缺失");
        UploadFile::load().expect("uploadFile.dll 导出函数缺失");
        FocusQqWindow::load().expect("FocusQQWindow2.dll 导出函数缺失");
    }

    #[test]
    fn point_and_rect_match_the_c_layout() {
        assert_eq!(std::mem::size_of::<Point>(), 8);
        assert_eq!(std::mem::size_of::<Rect>(), 16);
        assert_eq!(std::mem::align_of::<Point>(), 4);
    }

    /// 校验 Win64 下"按值返回结构体"的 ABI：`rect()` 必须原样回传四个字段。
    #[test]
    fn struct_return_matches_the_win64_abi() {
        if find_dll("Vision.dll").is_err() {
            eprintln!("跳过：当前目录没有 Vision.dll");
            return;
        }
        let vision = VisionDll::load().expect("Vision.dll 加载失败");
        let rect = vision.rect(1, 2, 3, 4);
        assert_eq!((rect.left, rect.top, rect.right, rect.bottom), (1, 2, 3, 4));
    }

    /// `getVkKey` 读的是 C 字符串，必须传 NUL 结尾的 `char*`。
    /// （C# 版直接把 `Encoding.ASCII.GetBytes()` 的数组指针递过去，并没有保证结尾是 0。）
    #[test]
    fn get_vk_key_reads_a_nul_terminated_c_string() {
        if find_dll("InputEvent.dll").is_err() {
            eprintln!("跳过：当前目录没有 InputEvent.dll");
            return;
        }
        let input = InputEvent::load().expect("InputEvent.dll 加载失败");

        assert_eq!(input.get_vk_key("TAB"), 0x09); // VK_TAB
        assert_eq!(input.get_vk_key("CTRL"), 0x11); // VK_CONTROL
        assert_eq!(input.get_vk_key("ENTER"), 0x0D); // VK_RETURN
        assert_eq!(input.get_vk_key("c"), 0x43); // 单字符按大写字母返回
        assert_eq!(input.get_vk_key("NOPE"), 0); // 未知键名
    }
}
