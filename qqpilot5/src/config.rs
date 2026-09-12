//! 统一配置模块。
//!
//! QQPilot4 (C#) 把这几个文件散落在四个地方各自解析：
//!   * `Program.Main`      —— width / height / scale / scroll / withimage / autologin / ...
//!   * `GUIOperation.Init` —— scroll / autofocusing（与上面重复读一次 config.ini）
//!   * `Vision` 静态构造器 —— width / height / scale（第三次读 config.ini）
//!   * `Answer` 构造函数   —— modelname / server_url / api_key / extra.json / system.txt
//!
//! 这里合并成"一次加载、一个类型"，后续要加/改配置项只需要动这一个文件：
//!   1. 在 [`Config`] 里加字段；
//!   2. 在 [`Config::default`] 里写出厂默认值；
//!   3. 在 [`Config::load`] 里加一行读取。

use std::fmt::Display;
use std::fs;
use std::path::Path;
use std::str::FromStr;
use std::sync::OnceLock;

use configparser::ini::Ini;
use serde_json::{Map, Value};

use crate::log::{self, Level};

/// config.ini 的位置（相对于进程当前工作目录，与 C# 版一致）。
pub const CONFIG_FILE: &str = "config.ini";
/// 附加请求参数文件。
pub const EXTRA_FILE: &str = "extra.json";
/// 系统提示词文件。
pub const SYSTEM_PROMPT_FILE: &str = "system.txt";

/// config.ini 里的节名。
const SECTION: &str = "general";

/// 全部运行时配置。
#[derive(Debug, Clone)]
pub struct Config {
    // --- 窗口 / 坐标 ---
    /// config.ini 的 version，仅用于启动时打印。
    pub version: String,
    /// 自己的角色名，用于判断聊天记录里哪条消息是自己发的。
    pub name: String,
    /// 窗口逻辑宽度。
    pub width: i32,
    /// 窗口逻辑高度。
    pub height: i32,
    /// DPI 缩放系数（由 ScaleToINI.exe 写入）。
    pub scale: f32,

    // --- 交互节奏 ---
    /// 拖拽后等待的秒数。
    pub scroll: i32,
    /// 发送前按 Tab 的次数。
    pub tab_times: i32,

    // --- 行为开关 ---
    /// 是否允许发送图片。
    pub with_image: bool,
    /// 启动时是否尝试自动登录。
    pub auto_login: bool,
    /// 是否让 QQ 窗口保持置顶。
    pub auto_focusing: bool,
    /// 是否用 @ 区域代替聊天列表检测红点。
    pub at_detect: bool,
    /// 发送图片的百分比概率（0-100）。
    pub send_image_possibility: i32,

    // --- 模型 / 接口 ---
    /// 模型是否支持图片输入。
    pub is_vision_model: bool,
    /// 单次请求最多携带的图片数量。
    pub max_image_count: usize,
    /// 模型名。
    pub model_name: String,
    /// 接口地址：`builtin` / `ollama` / 自定义 base url。
    pub server_url: String,
    /// Bearer Token。
    pub api_key: String,
    /// HTTP 超时秒数。
    pub remote_server_timeout: u64,
    /// 强制使用 Ollama `/api/chat` 协议。
    pub force_ollama_api: bool,

    // --- 原本读自独立文件的内容，一并集中到这里 ---
    /// system.txt 的内容。
    pub system_prompt: String,
    /// extra.json 的内容，会合并进请求体。
    pub extra: Map<String, Value>,
}

impl Default for Config {
    /// 出厂默认值，与仓库里 config.ini 的取值保持一致。
    fn default() -> Self {
        Self {
            version: "1.5.19".to_string(),
            name: String::new(),
            width: 1285,
            height: 720,
            scale: 1.0,
            scroll: 5,
            tab_times: 8,
            with_image: false,
            auto_login: false,
            auto_focusing: true,
            at_detect: false,
            send_image_possibility: 86,
            is_vision_model: true,
            max_image_count: 12,
            model_name: "qwen3.5:0.8b".to_string(),
            server_url: "builtin".to_string(),
            api_key: String::new(),
            remote_server_timeout: 300,
            force_ollama_api: false,
            system_prompt: String::new(),
            extra: Map::new(),
        }
    }
}

impl Config {
    /// 读取 config.ini / extra.json / system.txt。
    ///
    /// 缺失的键会回落到默认值并打一条 WARN，不会像 C# 版那样直接抛异常退出。
    pub fn load(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        let mut ini = Ini::new();
        if let Err(e) = ini.load(path.to_string_lossy().to_string()) {
            log::print_with(
                format!("读取 {} 失败：{e}，本次全部使用默认配置", path.display()),
                Level::Error,
            );
        }

        let d = Self::default();
        Self {
            version: get_str(&ini, "version", &d.version),
            name: get_str(&ini, "name", &d.name),
            width: get_int(&ini, "width", d.width),
            height: get_int(&ini, "height", d.height),
            scale: get_num(&ini, "scale", d.scale),
            scroll: get_int(&ini, "scroll", d.scroll),
            tab_times: get_int(&ini, "tab_times", d.tab_times),
            with_image: get_bool(&ini, "withimage", d.with_image),
            auto_login: get_bool(&ini, "autologin", d.auto_login),
            auto_focusing: get_bool(&ini, "autofocusing", d.auto_focusing),
            at_detect: get_bool(&ini, "atdetect", d.at_detect),
            send_image_possibility: get_int(&ini, "sendimagepossibility", d.send_image_possibility),
            is_vision_model: get_bool(&ini, "isvisionmodel", d.is_vision_model),
            max_image_count: get_int(&ini, "maximagecount", d.max_image_count as i32).max(0)
                as usize,
            model_name: get_str(&ini, "modelname", &d.model_name),
            server_url: get_str(&ini, "server_url", &d.server_url),
            api_key: get_str(&ini, "api_key", &d.api_key),
            remote_server_timeout: get_int(
                &ini,
                "remote_server_timeout",
                d.remote_server_timeout as i32,
            )
            .max(1) as u64,
            force_ollama_api: get_bool(&ini, "forceollamaapi", d.force_ollama_api),
            system_prompt: load_system_prompt(SYSTEM_PROMPT_FILE),
            extra: load_extra(EXTRA_FILE),
        }
    }
}

static CONFIG: OnceLock<Config> = OnceLock::new();

/// 加载配置（整个进程只真正读一次盘）；重复调用返回同一份。
pub fn init() -> &'static Config {
    CONFIG.get_or_init(|| Config::load(CONFIG_FILE))
}

/// [`init`] 的别名，供各处随手取用。
pub fn get() -> &'static Config {
    init()
}

// --- INI 读取辅助 ---

/// 原样取出值：键不存在返回 `None`，`key =` 这种空值返回 `Some("")`。
fn raw(ini: &Ini, key: &str) -> Option<String> {
    ini.get(SECTION, key)
}

fn warn_missing(key: &str, default: &impl Display) {
    log::print_with(
        format!("[配置] config.ini 缺少 {key}，使用默认值 {default}"),
        Level::Warn,
    );
}

fn get_str(ini: &Ini, key: &str, default: &str) -> String {
    match raw(ini, key) {
        Some(v) => v,
        None => {
            warn_missing(key, &default);
            default.to_string()
        }
    }
}

fn parse_or_warn<T: FromStr + Display>(key: &str, value: &str, default: T) -> T {
    match value.trim().parse::<T>() {
        Ok(v) => v,
        Err(_) => {
            log::print_with(
                format!("[配置] {key} 的值 {value:?} 无法解析，使用默认值 {default}"),
                Level::Warn,
            );
            default
        }
    }
}

fn get_int(ini: &Ini, key: &str, default: i32) -> i32 {
    match raw(ini, key) {
        Some(v) => parse_or_warn(key, &v, default),
        None => {
            warn_missing(key, &default);
            default
        }
    }
}

fn get_num(ini: &Ini, key: &str, default: f32) -> f32 {
    match raw(ini, key) {
        Some(v) => parse_or_warn(key, &v, default),
        None => {
            warn_missing(key, &default);
            default
        }
    }
}

/// C# 版用 `value.Equals("true", OrdinalIgnoreCase)`，这里保持一致：
/// 只认 `true`/`TRUE`/`True`，`yes`/`1` 都算 false。
fn get_bool(ini: &Ini, key: &str, default: bool) -> bool {
    match raw(ini, key) {
        Some(v) => v.eq_ignore_ascii_case("true"),
        None => {
            warn_missing(key, &default);
            default
        }
    }
}

// --- 原本独立读取的两个文件 ---

/// 读取 system.txt，去掉 UTF-8 BOM（C# 的 `File.ReadAllText` 会自动去掉）。
fn load_system_prompt(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(s) => s.trim_start_matches('\u{feff}').to_string(),
        Err(_) => String::new(),
    }
}

/// 读取 extra.json 的顶层对象；文件缺失或格式错误时返回空表（与 C# 版一致，静默忽略）。
fn load_extra(path: &str) -> Map<String, Value> {
    let Ok(text) = fs::read_to_string(path) else {
        return Map::new();
    };
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(map)) => map,
        Ok(_) => {
            log::print_with(
                format!("[配置] {path} 的顶层不是 JSON 对象，已忽略"),
                Level::Warn,
            );
            Map::new()
        }
        Err(e) => {
            log::print_with(format!("[配置] 解析 {path} 失败：{e}，已忽略"), Level::Warn);
            Map::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn write_temp(name: &str, content: &str) -> PathBuf {
        let path = std::env::temp_dir().join(name);
        fs::write(&path, content).expect("写入临时配置文件失败");
        path
    }

    #[test]
    fn parses_shipped_style_config() {
        let path = write_temp(
            "qqpilot5-config-ok.ini",
            "[general]\nversion = 9.9.9\nname=neko\nwidth = 1285\nheight = 720\n\
             scale=1.25\nscroll = 9\ntab_times = 3\nwithimage = TRUE\nautologin = false\n\
             autofocusing = true\natdetect = true\nsendimagepossibility = 42\n\
             isvisionmodel = true\nmaximagecount = 7\nmodelname = qwen3.5:0.8b\n\
             server_url = builtin\napi_key = abcd\nremote_server_timeout = 300\n\
             forceollamaapi=true\n",
        );

        let config = Config::load(&path);
        assert_eq!(config.version, "9.9.9");
        assert_eq!(config.name, "neko");
        assert_eq!(config.width, 1285);
        assert_eq!(config.scale, 1.25);
        assert_eq!(config.scroll, 9);
        assert_eq!(config.tab_times, 3);
        assert!(config.with_image);
        assert!(!config.auto_login);
        assert!(config.auto_focusing);
        assert!(config.at_detect);
        assert_eq!(config.send_image_possibility, 42);
        assert!(config.is_vision_model);
        assert_eq!(config.max_image_count, 7);
        assert_eq!(config.model_name, "qwen3.5:0.8b");
        assert_eq!(config.server_url, "builtin");
        assert_eq!(config.api_key, "abcd");
        assert_eq!(config.remote_server_timeout, 300);
        assert!(config.force_ollama_api);
    }

    #[test]
    fn missing_keys_fall_back_to_defaults() {
        let path = write_temp("qqpilot5-config-sparse.ini", "[general]\nwidth = 900\n");
        let config = Config::load(&path);
        let default = Config::default();

        assert_eq!(config.width, 900);
        assert_eq!(config.height, default.height);
        assert_eq!(config.scroll, default.scroll);
        assert_eq!(config.model_name, default.model_name);
    }

    #[test]
    fn only_literal_true_counts_as_true() {
        // 与 C# 的 `value.Equals("true", OrdinalIgnoreCase)` 一致：yes / 1 都不算 true。
        let path = write_temp(
            "qqpilot5-config-bool.ini",
            "[general]\nwithimage = yes\nautologin = 1\natdetect = True\n",
        );
        let config = Config::load(&path);
        assert!(!config.with_image);
        assert!(!config.auto_login);
        assert!(config.at_detect);
    }

    #[test]
    fn unparsable_number_falls_back_to_default() {
        let path = write_temp("qqpilot5-config-badnum.ini", "[general]\nwidth = abc\n");
        let config = Config::load(&path);
        assert_eq!(config.width, Config::default().width);
    }

    #[test]
    fn missing_file_uses_all_defaults() {
        let path = std::env::temp_dir().join("qqpilot5-config-does-not-exist.ini");
        let _ = fs::remove_file(&path);
        let config = Config::load(&path);
        assert_eq!(config.width, Config::default().width);
        assert_eq!(config.server_url, Config::default().server_url);
    }
}
