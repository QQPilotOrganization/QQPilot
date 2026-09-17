//! GUI/输入操作封装（对应 C# 的 `GUIOperation.cs`）。
//!
//! 所有坐标都是"实际屏幕坐标"；相对坐标 -> 实际坐标的换算在
//! [`crate::positions`] 里完成，本模块只负责调用原生 DLL。

use std::thread;
use std::time::Duration;

use crate::clipboard;
use crate::config;
use crate::log::{self, Color};
use crate::native;
use crate::positions::{PointI, RectI};
use crate::sleep;

/// 对应 `GUIOperation.Init()`：设置 DPI 感知并按需加载 DLL。
pub fn init() -> bool {
    let success = native::input_event().dpi_awareness_prologue();
    if !success {
        println!("⚠️ 警告: DPI 感知设置失败（可能影响高分屏坐标精度）");
    }
    success
}

// --- 鼠标 ---
pub fn clear_input_section() {
    hot_key("ctrl", "a");
    sleep::ms(200);
    press_key("backspace");
    sleep::ms(200);
}

/// 对应 `GUIOperation.Click(x, y)`：先平滑移动再左键单击。
pub fn click(x: i32, y: i32) -> bool {
    native::input_event().smooth_mouse_goto(x as u32, y as u32);
    native::input_event().l_click(x as u32, y as u32)
}

/// 对应 `GUIOperation.getAreaCenter`。
///
pub fn get_area_center(area: RectI) -> PointI {
    let pos1 = area.0 + ((area.2 - area.0) % 2);
    let pos2 = area.1 + ((area.3 - area.1) % 2);
    (pos1, pos2)
}

/// 对应 `GUIOperation.ClickCenter`。
pub fn click_center(area: RectI) -> bool {
    let (x, y) = get_area_center(area);
    click(x, y)
}

/// 对应 `GUIOperation.Goto`。
pub fn goto(x: i32, y: i32) {
    native::input_event().smooth_mouse_goto(x as u32, y as u32);
}

/// 对应 `GUIOperation.GotoCenter`。
pub fn goto_center(area: RectI) {
    let (x, y) = get_area_center(area);
    goto(x, y);
}

/// 对应 `GUIOperation.ScrollDown`。
pub fn scroll_down(delta: i32) -> bool {
    native::input_event().scroll_down(delta)
}

/// 对应 `GUIOperation.DragFromToSimple`：按下 -> 移动到终点 -> 等待 `scroll` 秒 -> 抬起。
pub fn drag_from_to_simple(x1: i32, y1: i32, x2: i32, y2: i32) {
    let input = native::input_event();
    input.smooth_mouse_goto(x1 as u32, y1 as u32);
    thread::sleep(Duration::from_millis(100));
    input.mouse_down();
    thread::sleep(Duration::from_millis(100));
    input.smooth_mouse_goto(x2 as u32, y2 as u32);
    thread::sleep(Duration::from_secs(config::get().scroll.max(0) as u64));
    input.mouse_up();
}

// --- 键盘 ---

/// 对应 `GUIOperation.PressKey`：键名找不到时与 C# 一样直接中断。
pub fn press_key(key_name: &str) {
    let vk = native::input_event().get_vk_key(&key_name.to_uppercase());
    assert!(vk != 0, "未知键名: {key_name}");
    native::input_event().press(vk);
}

/// 对应 `GUIOperation.HotKey`。
pub fn hot_key(modifier: &str, key: &str) {
    let input = native::input_event();
    let mod_vk = input.get_vk_key(&modifier.to_uppercase());
    let key_vk = input.get_vk_key(&key.to_uppercase());
    assert!(mod_vk != 0, "未知修饰键: {modifier}");
    assert!(key_vk != 0, "未知按键: {key}");
    input.hot_key(mod_vk, key_vk);
}

/// 对应 `GUIOperation.Tab`。
pub fn tab() {
    press_key("TAB");
}

/// 对应 `GUIOperation.Focus_`。
pub fn focus() {
    native::focus_qq_window().focus(config::get().auto_focusing);
}

// --- 输入框 ---

/// 对应 `GUIOperation.SendText`：按 `[[NEXT]]` 分段，段内按 `\n` 分行发送。
pub fn send_text(text: &str, comment_section: RectI) {
    log::set_color(Color::Green);
    log::print(format!("发消息->{text}"));

    for item in text.split("[[NEXT]]") {
        log::print(format!("{item};"));

        let inputs: Vec<&str> = item.split('\n').collect();
        for line in &inputs[..inputs.len() - 1] {
            paste_text_to_section(line, comment_section);
            press_key("ENTER");
            thread::sleep(Duration::from_millis(200));
        }

        paste_text_to_section(inputs[inputs.len() - 1], comment_section);
        click_center(comment_section);
        thread::sleep(Duration::from_millis(200));
        hot_key("ctrl", "enter");
    }
}

/// 对应 `GUIOperation.PasteTextToSection`。
fn paste_text_to_section(text: &str, section: RectI) {
    clipboard::set_text(text);
    thread::sleep(Duration::from_millis(200));
    click_center(section);
    hot_key("ctrl", "v");
    thread::sleep(Duration::from_millis(800));
}
