//! 控制台日志与颜色（对应 C# 的 `Log.cs`）。
//!
//! C# 用 `Console.ForegroundColor` / `Console.ResetColor()`；这里直接调
//! Win32 `SetConsoleTextAttribute`，保持同样的语义（含"每次输出后自动恢复默认色"）。

use std::fs::OpenOptions;
use std::io::Write;
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Console::{
    CONSOLE_SCREEN_BUFFER_INFO, GetConsoleScreenBufferInfo, GetStdHandle, STD_OUTPUT_HANDLE,
    SetConsoleOutputCP, SetConsoleTextAttribute,
};

use chrono::Local;

/// 对应 `System.ConsoleColor`。取值即 Win32 高 4 位/低 4 位的颜色索引。
/// 完整保留调色板（虽然当前只用到其中几种），便于按 C# 版继续调色。
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum Color {
    Black = 0,
    DarkBlue = 1,
    DarkGreen = 2,
    DarkCyan = 3,
    DarkRed = 4,
    DarkMagenta = 5,
    DarkYellow = 6,
    Gray = 7,
    DarkGray = 8,
    Blue = 9,
    Green = 10,
    Cyan = 11,
    Red = 12,
    Magenta = 13,
    Yellow = 14,
    White = 15,
}

/// 对应 C# 的 `Log.Stat`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Normal,
    Warn,
    Error,
}

fn stdout_handle() -> HANDLE {
    // SAFETY: 只是取当前进程的标准输出句柄，不会失败到未定义行为。
    unsafe { GetStdHandle(STD_OUTPUT_HANDLE) }
}

/// 进程启动时的原始控制台属性，用于 `reset()`。
fn original_attributes() -> u16 {
    static ORIGINAL: OnceLock<u16> = OnceLock::new();
    *ORIGINAL.get_or_init(|| {
        let handle = stdout_handle();
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return Color::Gray as u16;
        }
        let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
        // SAFETY: info 是栈上已初始化、生命周期覆盖本次调用的结构体。
        if unsafe { GetConsoleScreenBufferInfo(handle, &mut info) } != 0 {
            info.wAttributes
        } else {
            Color::Gray as u16
        }
    })
}

fn apply(attributes: u16) {
    let handle = stdout_handle();
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        return;
    }
    // SAFETY: 句柄来自 GetStdHandle，attributes 是合法的颜色位组合。
    unsafe { SetConsoleTextAttribute(handle, attributes) };
}

/// 对应 `Console.OutputEncoding = Encoding.UTF8`。
pub fn enable_utf8_output() {
    // SAFETY: 参数是合法的代码页编号。
    unsafe { SetConsoleOutputCP(65001) };
}

/// 对应 `Log.SetColor` / `Console.ForegroundColor = color`。
pub fn set_color(color: Color) {
    let attributes = original_attributes() & 0xFFF0 | (color as u16);
    apply(attributes);
}

/// 对应 `Console.BackgroundColor = color`。
pub fn set_background_color(color: Color) {
    let attributes = original_attributes() & 0xFF0F | ((color as u16) << 4);
    apply(attributes);
}

/// 对应 `Console.ResetColor()`。
pub fn reset() {
    apply(original_attributes());
}

/// 当前控制台缓冲区宽度，对应 `Console.BufferWidth`；取不到时返回 80。
pub fn buffer_width() -> i32 {
    let handle = stdout_handle();
    if handle == INVALID_HANDLE_VALUE || handle.is_null() {
        return 80;
    }
    let mut info = CONSOLE_SCREEN_BUFFER_INFO::default();
    // SAFETY: 同 original_attributes。
    if unsafe { GetConsoleScreenBufferInfo(handle, &mut info) } != 0 {
        i32::from(info.dwSize.X)
    } else {
        80
    }
}

/// 对应 `Log.Print(target)`（默认 `Stat.NORMAL`）。
pub fn print(message: impl AsRef<str>) {
    print_with(message, Level::Normal);
}

/// 对应 `Log.Print(target, Log.Stat.WARN)`。
pub fn warn(message: impl AsRef<str>) {
    print_with(message, Level::Warn);
}

/// 对应 `Log.Print(target, Log.Stat.ERROR)`。
pub fn error(message: impl AsRef<str>) {
    print_with(message, Level::Error);
}

/// 对应 `Log.Print`：打时间戳、按等级变色、写控制台并追加到 log.txt，最后恢复默认色。
pub fn print_with(message: impl AsRef<str>, level: Level) {
    let timestamp = Local::now().format("%H:%M:%S");
    let mut output = format!("[{timestamp}]{}", message.as_ref());

    match level {
        Level::Normal => {}
        Level::Warn => {
            output = format!("[WARN]{output}");
            set_color(Color::Yellow);
        }
        Level::Error => {
            output = format!("[ERR]{output}");
            set_color(Color::Red);
        }
    }

    println!("{output}");

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("log.txt") {
        let _ = writeln!(file, "{output}");
    }

    reset();
}
