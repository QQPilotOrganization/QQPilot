//! 剪贴板（对应 C# 里 `TextCopy` 的用法）。
//!
//! 这里刻意**复用同一个** `Clipboard` 实例：Windows 上剪贴板内容归属于它的"所有者窗口"，
//! 所有者窗口被销毁时剪贴板会被清空。C# 版是 `Clipboard clipboard = new();` 之后一直握着
//! 这个对象（设置空串 -> QQ 复制 -> 读取），所以这里也必须让它活到进程结束，
//! 否则"写入 -> Ctrl+V"之间内容就没了。

use std::sync::{Mutex, MutexGuard, OnceLock};

use arboard::Clipboard;

use crate::log;

static CLIPBOARD: OnceLock<Mutex<Option<Clipboard>>> = OnceLock::new();

fn clipboard() -> Option<MutexGuard<'static, Option<Clipboard>>> {
    let mutex = CLIPBOARD.get_or_init(|| Mutex::new(Clipboard::new().ok()));
    match mutex.lock() {
        Ok(guard) => Some(guard),
        Err(e) => {
            log::error(format!("剪贴板状态锁已损坏: {e}"));
            None
        }
    }
}

/// 对应 `ClipboardService.SetText(text)`。
pub fn set_text(text: &str) -> bool {
    let Some(mut guard) = clipboard() else {
        return false;
    };
    let Some(clipboard) = guard.as_mut() else {
        log::error("打开剪贴板失败");
        return false;
    };
    match clipboard.set_text(text.to_string()) {
        Ok(()) => true,
        Err(e) => {
            log::error(format!("写入剪贴板失败: {e}"));
            false
        }
    }
}

/// 对应 `clipboard.GetText() ?? ""`：取不到内容时返回空串。
pub fn get_text() -> String {
    let Some(mut guard) = clipboard() else {
        return String::new();
    };
    let Some(clipboard) = guard.as_mut() else {
        log::error("打开剪贴板失败");
        return String::new();
    };
    clipboard.get_text().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    /// 临时验证：跑完即删。
    /// 覆盖运行期会污染用户剪贴板，因此默认忽略；需要时用
    /// `cargo test clipboard -- --ignored --nocapture` 手动跑。
    #[test]
    #[ignore = "会覆盖系统剪贴板"]
    fn probe_clipboard_roundtrip() {
        let saved = Clipboard::new().ok().and_then(|mut c| c.get_text().ok());

        set_text("qqpilot5-clipboard-probe-123");
        thread::sleep(Duration::from_millis(300));
        assert_eq!(get_text(), "qqpilot5-clipboard-probe-123");

        // 模拟"外部程序（QQ）抢走剪贴板后再读"
        if let Ok(mut other) = Clipboard::new() {
            other.set_text("external-writer".to_string()).unwrap();
        }
        thread::sleep(Duration::from_millis(300));
        assert_eq!(get_text(), "external-writer");

        if let Some(text) = saved
            && let Ok(mut c) = Clipboard::new()
        {
            let _ = c.set_text(text);
        }
    }
}
