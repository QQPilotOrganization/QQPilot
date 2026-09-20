//! QQPilot 设置界面（`QQPilotGUISharp` 的 Rust 移植版）。
//!
//! 控件、坐标、尺寸全部照搬 C# 的 `Form1.Designer.cs`，行为照搬 `Form1.cs`：
//! 启动时读 config.ini / system.txt / tokencount.txt 填充界面，
//! 点「保存设置」或关闭窗口时写回。
//!
//! AntdUI 控件到 native-windows-gui 的对应关系：
//!   * `Input`        -> `TextInput`（多行用 `TextBox`）
//!   * `InputNumber`  -> `TextInput` + `ES_NUMBER`（上下限在读写时夹紧）
//!   * `Switch`       -> `CheckBox`
//!   * `Select`       -> `ComboBox`
//!   * `Slider`       -> `TrackBar`（数值显示在左侧的标签里）
//!   * `ButtonShadow` -> `Button`
//!   * `Tooltip`      -> 普通 `Label`（NWG 没有可点击的常显提示控件）

mod config;
mod localization;

use std::process::Command;

use native_windows_derive::NwgUi;
use native_windows_gui as nwg;
use nwg::NativeUi;
use winapi::shared::windef::RECT;
use winapi::um::winuser::{
    CB_GETITEMHEIGHT, CB_SETITEMHEIGHT, GetClientRect, GetWindowRect, SendMessageW,
};

use config::{NUL_STR, Settings};

/// 对应 C# 的 `Form1.ClientSize`。
const CLIENT_WIDTH: u32 = 1075;
const CLIENT_HEIGHT: u32 = 489;
/// 对应 `Form1_Load` 里的右下角留白。
const MARGIN_RIGHT: i32 = 20;
const MARGIN_BOTTOM: i32 = 50;

/// 「框选消息时长」的上下限，对应 `Scroll_ValueChanged`。
const SCROLL_MIN: i32 = 1;
const SCROLL_MAX: i32 = 200;
/// 「解析图片数」的上下限，对应 `MaxImageCount_ValueChanged`。
const MAX_IMAGE_MIN: i32 = 0;
const MAX_IMAGE_MAX: i32 = 4;
/// 对应 Designer 里 `winWidth.Minimum` / `winHeight.Maximum` / `RemoteServerTimeout.Minimum`。
const WINDOW_WIDTH_MIN: i32 = 1280;
const WINDOW_HEIGHT_MAX: i32 = 720;
const TIMEOUT_MIN: i32 = 60;

#[derive(Default, NwgUi)]
pub struct SettingsApp {
    #[nwg_control(size: (CLIENT_WIDTH as i32, CLIENT_HEIGHT as i32), position: (0, 0),
                  title: localization::text("option.title"), flags: "WINDOW|VISIBLE", icon: Some(&data.icon))]
    #[nwg_events(OnInit: [SettingsApp::on_init], OnWindowClose: [SettingsApp::on_close])]
    window: nwg::Window,

    #[nwg_resource(source_bin: Some(include_bytes!("../option.ico")))]
    icon: nwg::Icon,

    // ---- 版本 / 用户名 ----
    #[nwg_control(text: localization::text("option.version"), size: (46, 22), position: (11, 7))]
    label_version: nwg::Label,

    #[nwg_control(text: "", size: (449, 22), position: (86, 7))]
    vname: nwg::Label,

    #[nwg_control(text: localization::text("option.user.name"), size: (70, 22), position: (9, 33))]
    label_user_name: nwg::Label,

    #[nwg_control(size: (218, 36), position: (83, 26))]
    #[nwg_events(OnTextInput: [SettingsApp::guard_empty_text])]
    user_name: nwg::TextInput,

    #[nwg_control(text: localization::text("option.user.name.hint"), size: (168, 38), position: (304, 24))]
    label_user_name_hint: nwg::Label,

    // ---- 窗口尺寸 / token 计数 ----
    #[nwg_control(text: localization::text("option.window.width"), size: (70, 22), position: (11, 66))]
    label_window_width: nwg::Label,

    #[nwg_control(size: (84, 34), position: (86, 59), flags: "VISIBLE|TAB_STOP|NUMBER")]
    win_width: nwg::TextInput,

    #[nwg_control(text: localization::text("option.window.height"), size: (70, 22), position: (174, 66))]
    label_window_height: nwg::Label,

    #[nwg_control(size: (84, 34), position: (248, 59), flags: "VISIBLE|TAB_STOP|NUMBER")]
    win_height: nwg::TextInput,

    #[nwg_control(text: localization::text("option.token.count"), size: (70, 22), position: (355, 66), h_align: nwg::HTextAlign::Center)]
    label_token_count: nwg::Label,

    #[nwg_control(text: "0", size: (59, 22), position: (432, 66))]
    token_count: nwg::Label,

    #[nwg_control(text: localization::text("option.reset.token"), size: (111, 32), position: (416, 93))]
    #[nwg_events(OnButtonClick: [SettingsApp::reset_token_count])]
    button_reset_token: nwg::Button,

    // ---- 图片解析 ----
    #[nwg_control(text: localization::text("option.max.image.count"), size: (70, 22), position: (11, 103))]
    label_max_image_count: nwg::Label,

    #[nwg_control(size: (84, 34), position: (86, 97), flags: "VISIBLE|TAB_STOP|NUMBER")]
    #[nwg_events(OnTextInput: [SettingsApp::on_max_image_count_changed])]
    max_image_count: nwg::TextInput,

    #[nwg_control(text: localization::text("option.max.image.hint"), size: (227, 38), position: (174, 93))]
    label_max_image_hint: nwg::Label,

    // ---- 模型 / 接口 ----
    #[nwg_control(text: localization::text("option.model.name"), size: (70, 22), position: (11, 139))]
    label_model_name: nwg::Label,

    #[nwg_control(size: (458, 36), position: (86, 131))]
    #[nwg_events(OnTextInput: [SettingsApp::guard_empty_text])]
    model_name: nwg::TextInput,

    #[nwg_control(text: localization::text("option.vision.model"), size: (70, 22), position: (11, 176))]
    label_vision_model: nwg::Label,

    #[nwg_control(text: "", size: (78, 33), position: (86, 170), flags: "VISIBLE|TAB_STOP")]
    is_vision_model: nwg::CheckBox,

    #[nwg_control(text: localization::text("option.api.key"), size: (70, 22), position: (11, 215))]
    label_api_key: nwg::Label,

    #[nwg_control(size: (459, 36), position: (84, 207), password: Some('·'))]
    #[nwg_events(OnTextInput: [SettingsApp::guard_empty_text])]
    api_key: nwg::TextInput,

    #[nwg_control(text: localization::text("option.server"), size: (70, 22), position: (11, 262))]
    label_server: nwg::Label,

    #[nwg_control(size: (111, 36), position: (86, 254), flags: "VISIBLE|TAB_STOP",
                  collection: vec!["ollama".to_string(), localization::text("option.server.builtin").to_string(), localization::text("option.server.custom").to_string()])]
    #[nwg_events(OnComboxBoxSelection: [SettingsApp::on_server_changed])]
    server_name: nwg::ComboBox<String>,

    #[nwg_control(size: (343, 36), position: (200, 254))]
    server_url: nwg::TextInput,

    #[nwg_control(text: localization::text("option.extra.json"), size: (111, 32), position: (200, 297))]
    #[nwg_events(OnButtonClick: [SettingsApp::open_extra_json])]
    button_extra_json: nwg::Button,

    #[nwg_control(text: localization::text("option.force.ollama"), size: (136, 22), position: (336, 302))]
    label_force_ollama: nwg::Label,

    #[nwg_control(text: "", size: (78, 33), position: (465, 297), flags: "VISIBLE|TAB_STOP")]
    force_ollama_api: nwg::CheckBox,

    // ---- 交互节奏 ----
    #[nwg_control(text: localization::text("option.scroll"), size: (90, 22), position: (11, 302))]
    label_scroll: nwg::Label,

    #[nwg_control(size: (84, 34), position: (105, 294), flags: "VISIBLE|TAB_STOP|NUMBER")]
    #[nwg_events(OnTextInput: [SettingsApp::on_scroll_changed])]
    scroll: nwg::TextInput,

    #[nwg_control(text: localization::text("option.with.image"), size: (70, 22), position: (11, 341))]
    label_with_image: nwg::Label,

    #[nwg_control(text: "", size: (78, 33), position: (86, 336), flags: "VISIBLE|TAB_STOP")]
    with_image: nwg::CheckBox,

    #[nwg_control(text: localization::text("option.auto.login"), size: (90, 22), position: (174, 341))]
    label_auto_login: nwg::Label,

    #[nwg_control(text: "", size: (78, 33), position: (254, 336), flags: "VISIBLE|TAB_STOP")]
    auto_login: nwg::CheckBox,

    #[nwg_control(text: localization::text("option.auto.focusing"), size: (136, 22), position: (336, 341))]
    label_auto_focusing: nwg::Label,

    #[nwg_control(text: "", size: (78, 33), position: (465, 336), flags: "VISIBLE|TAB_STOP")]
    auto_focusing: nwg::CheckBox,

    // ---- 发送图片概率 ----
    #[nwg_control(text: localization::text("option.send.image.possibility"), size: (118, 22), position: (9, 378))]
    label_possibility: nwg::Label,

    #[nwg_control(size: (424, 33), position: (120, 373), range: Some(0..100), pos: Some(0),
                  flags: "VISIBLE|TAB_STOP|HORIZONTAL")]
    #[nwg_events(OnHorizontalScroll: [SettingsApp::on_possibility_changed])]
    send_image_possibility: nwg::TrackBar,

    #[nwg_control(text: localization::text("option.at.detect"), size: (70, 22), position: (11, 413))]
    label_at_detect: nwg::Label,

    #[nwg_control(text: localization::text("option.sleep"), size: (180, 22), position: (200, 413)) ]
    label_sleep: nwg::Label,

    #[nwg_control(text: "", size: (134, 22), position: (360, 413),flags: "VISIBLE|TAB_STOP|NUMBER")]
    sleep: nwg::TextInput,

    #[nwg_control(text: "", size: (78, 33), position: (86, 408), flags: "VISIBLE|TAB_STOP")]
    at_detect: nwg::CheckBox,

    // ---- 超时 / tab 次数 ----
    #[nwg_control(text: localization::text("option.timeout"), size: (134, 22), position: (11, 446))]
    label_timeout: nwg::Label,

    #[nwg_control(size: (84, 34), position: (174, 439), flags: "VISIBLE|TAB_STOP|NUMBER")]
    remote_server_timeout: nwg::TextInput,

    #[nwg_control(text: localization::text("option.tab.times"), size: (134, 22), position: (291, 446))]
    label_tab_times: nwg::Label,

    #[nwg_control(size: (111, 36), position: (433, 437), flags: "VISIBLE|TAB_STOP",
                  collection: vec!["7".to_string(), "8".to_string()])]
    tab_times: nwg::ComboBox<String>,

    // ---- 提示文本 / 保存 ----
    #[nwg_control(text: localization::text("option.system.text"), size: (82, 22), position: (552, 7))]
    label_system_text: nwg::Label,

    #[nwg_control(size: (514, 439), position: (552, 39), flags: "VISIBLE|TAB_STOP|VSCROLL")]
    system_text: nwg::TextBox,

    #[nwg_control(text: localization::text("option.save"), size: (145, 34), position: (916, 7))]
    #[nwg_events(OnButtonClick: [SettingsApp::save_config])]
    button_save: nwg::Button,
}

impl SettingsApp {
    /// 对应 `Form1_Load`：把窗口摆到右下角、客户区定为 1075x489，然后载入配置。
    fn on_init(&self) {
        // nwg 的 set_size 收的是"客户区"尺寸，内部会按 DPI 换算再加上边框，
        // 正好对应 C# 的 ClientSize。
        self.window.set_size(CLIENT_WIDTH, CLIENT_HEIGHT);

        // C# 是 `Left = Screen.Width - Width - 20`（Width 为窗口外框宽），
        // 这里同理：全部按逻辑像素算。
        let (frame_width, frame_height) = self.frame_size();
        let (client_width, client_height) = self.window.size();
        let scale = nwg::scale_factor();
        let screen_width = (nwg::Monitor::width() as f64 / scale) as i32;
        let screen_height = (nwg::Monitor::height() as f64 / scale) as i32;

        let x = screen_width - (client_width as i32 + frame_width) - MARGIN_RIGHT;
        let y = screen_height - (client_height as i32 + frame_height) - MARGIN_BOTTOM;
        self.window.set_position(x.max(0), y.max(0));

        self.load_settings();
    }

    /// 对应 C# 的终结器 `~Form1()`：关闭窗口时保存。
    fn on_close(&self) {
        self.save_config();
        nwg::stop_thread_dispatch();
    }

    /// 窗口外框比客户区大出来的部分（逻辑像素）。
    fn frame_size(&self) -> (i32, i32) {
        let Some(hwnd) = self.window.handle.hwnd() else {
            return (0, 0);
        };
        if hwnd.is_null() {
            return (0, 0);
        }

        let (mut outer, mut client) = (
            RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
            RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
        );
        // SAFETY: hwnd 来自已创建的窗口；两个 RECT 都是本函数栈上的有效内存。
        unsafe {
            GetWindowRect(hwnd, &mut outer);
            GetClientRect(hwnd, &mut client);
        }

        let scale = nwg::scale_factor();
        let width = (outer.right - outer.left) - (client.right - client.left);
        let height = (outer.bottom - outer.top) - (client.bottom - client.top);
        (
            ((width as f64 / scale).round() as i32).max(0),
            ((height as f64 / scale).round() as i32).max(0),
        )
    }

    /// 对应 `Form1_Load` 里把 config.ini 读进各个控件的那一段。
    fn load_settings(&self) {
        let settings = Settings::load();

        self.vname.set_text(&settings.version);
        self.user_name.set_text(&settings.name);
        self.win_width.set_text(&settings.width.to_string());
        self.win_height.set_text(&settings.height.to_string());
        self.max_image_count
            .set_text(&settings.max_image_count.to_string());
        self.model_name.set_text(&settings.model_name);
        self.is_vision_model
            .set_check_state(check_state(settings.is_vision_model));
        self.api_key.set_text(&settings.api_key);
        self.scroll.set_text(&settings.scroll.to_string());
        self.with_image
            .set_check_state(check_state(settings.with_image));
        self.auto_login
            .set_check_state(check_state(settings.auto_login));
        self.auto_focusing
            .set_check_state(check_state(settings.auto_focusing));
        self.at_detect
            .set_check_state(check_state(settings.at_detect));
        self.remote_server_timeout
            .set_text(&settings.remote_server_timeout.to_string());
        self.force_ollama_api
            .set_check_state(check_state(settings.force_ollama_api));

        self.send_image_possibility
            .set_pos(settings.send_image_possibility.clamp(0, 100) as usize);
        self.refresh_possibility_label();

        // Win32 下拉框的显示区高度由字体决定，不会理会 builder 里给的高度，
        // 而 C# 那两个 Select 都是 36 高，所以这里手动对齐。
        fit_combo_height(&self.server_name, 36);
        fit_combo_height(&self.tab_times, 36);

        // C# 先设置下拉的 SelectedIndex（会触发一次联动），随后又用配置里的原文覆盖输入框，
        // 所以最终一定是"输入框显示配置原文、可用性按下拉选择"。
        let server_index = server_index(&settings.server_url);
        self.server_name.set_selection(Some(server_index));
        self.server_url.set_text(&settings.server_url);
        self.set_server_url_enabled(server_index == CUSTOM_SERVER_INDEX);

        // tab_times 只允许 7 或 8，其它值按 C# 的处理落到 8。
        self.tab_times
            .set_selection(Some(if settings.tab_times == 7 { 0 } else { 1 }));

        self.system_text.set_text(&config::load_system_text());

        self.token_count
            .set_text(&match config::load_token_count() {
                Some(text) => text,
                None => config::reset_token_count().unwrap_or_else(|_| "0".to_string()),
            });
        self.sleep.set_text(&settings.sleep.to_string());
    }

    /// 对应 `Form1.SaveConfig`：把界面上的值攒成 [`Settings`] 并写回文件。
    fn save_config(&self) {
        let settings = Settings {
            version: self.vname.text(),
            name: self.user_name.text(),
            // Designer 里这几个 InputNumber 带上下限，这里在写盘时补上夹紧。
            width: read_int(&self.win_width, Settings::default().width).max(WINDOW_WIDTH_MIN),
            height: read_int(&self.win_height, Settings::default().height).min(WINDOW_HEIGHT_MAX),
            max_image_count: read_int(&self.max_image_count, Settings::default().max_image_count)
                .clamp(MAX_IMAGE_MIN, MAX_IMAGE_MAX),
            model_name: self.model_name.text(),
            is_vision_model: self.is_vision_model.check_state() == nwg::CheckBoxState::Checked,
            api_key: self.api_key.text(),
            server_url: self.server_url.text(),
            scroll: read_int(&self.scroll, Settings::default().scroll)
                .clamp(SCROLL_MIN, SCROLL_MAX),
            with_image: self.with_image.check_state() == nwg::CheckBoxState::Checked,
            auto_login: self.auto_login.check_state() == nwg::CheckBoxState::Checked,
            auto_focusing: self.auto_focusing.check_state() == nwg::CheckBoxState::Checked,
            send_image_possibility: self.send_image_possibility.pos() as i32,
            at_detect: self.at_detect.check_state() == nwg::CheckBoxState::Checked,
            remote_server_timeout: read_int(
                &self.remote_server_timeout,
                Settings::default().remote_server_timeout,
            )
            .max(TIMEOUT_MIN),
            force_ollama_api: self.force_ollama_api.check_state() == nwg::CheckBoxState::Checked,
            // 对应 `TabTimes.SelectedIndex == 0 ? 7 : 8`
            tab_times: if self.tab_times.selection() == Some(0) {
                7
            } else {
                8
            },
            sleep: read_int(&self.sleep, 0) as u32,
        };

        if let Err(e) = settings.save(&self.system_text.text()) {
            nwg::modal_error_message(&self.window, localization::text("option.title"), &e);
        }
    }
    /// 对应 `IsNULL`：这几个输入框不允许为空，空的话填上占位符 `_`。
    ///
    /// C# 是逐次按键触发的，这里保持同样时机。
    fn guard_empty_text(&self) {
        for input in [&self.user_name, &self.model_name, &self.api_key] {
            if input.text().trim().is_empty() {
                input.set_text(NUL_STR);
            }
        }
    }

    /// 对应 `MaxImageCount_ValueChanged`：把值夹到 0..4。
    fn on_max_image_count_changed(&self) {
        clamp_text(&self.max_image_count, MAX_IMAGE_MIN, MAX_IMAGE_MAX);
    }

    /// 对应 `Scroll_ValueChanged`：把值夹到 1..200。
    fn on_scroll_changed(&self) {
        clamp_text(&self.scroll, SCROLL_MIN, SCROLL_MAX);
    }

    /// 对应 `SendImagePossibility` 的取值变化：把数值回显到左侧标签。
    fn on_possibility_changed(&self) {
        self.refresh_possibility_label();
    }

    fn refresh_possibility_label(&self) {
        self.label_possibility.set_text(&format!(
            "{}: {}",
            localization::text("option.send.image.possibility"),
            self.send_image_possibility.pos()
        ));
    }

    /// 对应 `ServerName_SelectedIndexChanged`。
    fn on_server_changed(&self) {
        match self.server_name.selection() {
            Some(0) => {
                self.server_url.set_text("ollama");
                self.set_server_url_enabled(false);
            }
            Some(1) => {
                self.server_url.set_text("builtin");
                self.set_server_url_enabled(false);
            }
            _ => self.set_server_url_enabled(true),
        }
    }

    fn set_server_url_enabled(&self, enabled: bool) {
        self.server_url.set_enabled(enabled);
        self.force_ollama_api.set_enabled(enabled);
    }

    /// 对应 `ResetTokenConuter`。
    fn reset_token_count(&self) {
        match config::reset_token_count() {
            Ok(text) => self.token_count.set_text(&text),
            Err(e) => {
                nwg::modal_error_message(&self.window, localization::text("option.title"), &e);
            }
        }
    }

    /// 对应 `buttonShadow3_Click`：用记事本打开 extra.json。
    ///
    /// C# 版会 `WaitForExit()` 把消息循环一起卡住；这里改成不等待，设置窗口保持可用。
    fn open_extra_json(&self) {
        if let Err(e) = Command::new("notepad").arg("extra.json").spawn() {
            nwg::modal_error_message(
                &self.window,
                localization::text("option.title"),
                &format!("{}: {e}", localization::text("error.extra.json.open")),
            );
        }
    }
}

/// 由下拉框选项推导出来的"自定义服务器"下标，对应 `ServerName.Items` 的第三项。
const CUSTOM_SERVER_INDEX: usize = 2;

/// 对应 `general["server_url"].ToLower() switch { "ollama" => 0, "builtin" => 1, _ => 2 }`。
fn server_index(server_url: &str) -> usize {
    match server_url.to_lowercase().as_str() {
        "ollama" => 0,
        "builtin" => 1,
        _ => CUSTOM_SERVER_INDEX,
    }
}

/// 把下拉框的整体高度调成指定值（逻辑像素）。
///
/// Win32 下拉框的显示区高度由字体决定，builder 里给的高度不会生效，
/// 而 C# 那两个 `Select` 都是 36 高。`CB_SETITEMHEIGHT` 传 `wParam = -1`
/// 只能设置"显示区"高度，总高度还要加上上下边框，所以这里先量出边框再补。
fn fit_combo_height(combo: &nwg::ComboBox<String>, logical_height: i32) {
    let Some(hwnd) = combo.handle.hwnd() else {
        return;
    };
    if hwnd.is_null() {
        return;
    }
    let target = (logical_height as f64 * nwg::scale_factor()).round() as isize;

    // 全部按物理像素量，别和 nwg 的逻辑像素口径混在一起。
    let mut client = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    // SAFETY: hwnd 来自已创建的下拉框，client 是本函数栈上的有效内存。
    unsafe {
        GetClientRect(hwnd, &mut client);
    }
    let total = (client.bottom - client.top) as isize;
    // SAFETY: 同上，CB_GETITEMHEIGHT 只收发整数。
    let item = unsafe { SendMessageW(hwnd, CB_GETITEMHEIGHT, usize::MAX, 0) } as isize;
    let border = (total - item).max(0);

    // SAFETY: 同上。
    unsafe {
        SendMessageW(hwnd, CB_SETITEMHEIGHT, usize::MAX, target - border);
    }
}

fn check_state(value: bool) -> nwg::CheckBoxState {
    if value {
        nwg::CheckBoxState::Checked
    } else {
        nwg::CheckBoxState::Unchecked
    }
}

/// 读取数字输入框；内容不是数字时回落到默认值（C# 里 AntdUI 会保证它始终是数字）。
fn read_int(input: &nwg::TextInput, fallback: i32) -> i32 {
    input.text().trim().parse::<i32>().unwrap_or(fallback)
}

/// 把输入框里的数字夹进范围；不是数字就原样留着，等用户继续输。
fn clamp_text(input: &nwg::TextInput, min: i32, max: i32) {
    let Ok(value) = input.text().trim().parse::<i32>() else {
        return;
    };
    let clamped = value.clamp(min, max);
    if clamped != value {
        input.set_text(&clamped.to_string());
    }
}

fn main() {
    #[allow(deprecated)]
    unsafe {
        nwg::set_dpi_awareness();
    }

    let ui_init_failed = localization::text("error.ui.init");
    nwg::init().expect(ui_init_failed);

    #[allow(deprecated)]
    unsafe {
        nwg::set_dpi_awareness();
    }

    // 界面文案全是中文，用微软雅黑比系统默认字体好看；设置失败就沿用系统默认。
    let _ = nwg::Font::set_global_family("Microsoft YaHei UI");

    let ui_build_failed = localization::text("error.ui.build");
    let _app = SettingsApp::build_ui(Default::default()).expect(ui_build_failed);
    nwg::dispatch_thread_events();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_index_matches_csharp_switch() {
        assert_eq!(server_index("ollama"), 0);
        assert_eq!(server_index("OLLAMA"), 0);
        assert_eq!(server_index("builtin"), 1);
        assert_eq!(server_index("BuiltIn"), 1);
        assert_eq!(
            server_index("http://192.168.1.100:8000/v1"),
            CUSTOM_SERVER_INDEX
        );
    }
}
