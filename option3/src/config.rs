//! 配置文件读写层：`config.ini` / `system.txt` / `tokencount.txt`。
//!
//! 对应 C# 的 `Form1_Load`（读取并填充界面）与 `Form1.SaveConfig`（写回）。
//! 界面代码只跟 [`Settings`] 打交道，不直接碰文件。

use std::fs;
use std::path::Path;

use configparser::ini::{Ini, WriteOptions};

/// 主配置文件。
pub const CONFIG_FILE: &str = "config.ini";
/// 系统提示词文件。
pub const SYSTEM_FILE: &str = "system.txt";
/// token 累计用量的记录文件。
pub const TOKEN_FILE: &str = "tokencount.txt";
/// config.ini 里 `[general]` 节的名字。
const SECTION: &str = "general";
/// 对应 C# 的 `NULSTr`：文本框内容为空时用它占位，避免写出空值。
pub const NUL_STR: &str = "_";

/// 界面上可编辑的全部配置项。
///
/// 字段严格对应 C# 版 `Form1_Load` 读取、`SaveConfig` 写回的那批键；
/// `scale` / `nt_data` / `system` 等键不归这个界面管，写回时会原样保留。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub version: String,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub max_image_count: i32,
    pub model_name: String,
    pub is_vision_model: bool,
    pub api_key: String,
    pub server_url: String,
    pub scroll: i32,
    pub with_image: bool,
    pub auto_login: bool,
    pub auto_focusing: bool,
    pub send_image_possibility: i32,
    pub at_detect: bool,
    pub remote_server_timeout: i32,
    pub force_ollama_api: bool,
    /// 只允许 7 或 8。
    pub tab_times: i32,
}

impl Default for Settings {
    /// 出厂默认值，与仓库里 config.ini 的取值一致。
    fn default() -> Self {
        Self {
            version: "1.5.19".to_string(),
            name: String::new(),
            width: 1285,
            height: 720,
            max_image_count: 12,
            model_name: "qwen3.5:0.8b".to_string(),
            is_vision_model: true,
            api_key: String::new(),
            server_url: "builtin".to_string(),
            scroll: 5,
            with_image: false,
            auto_login: false,
            auto_focusing: true,
            send_image_possibility: 86,
            at_detect: false,
            remote_server_timeout: 300,
            force_ollama_api: false,
            tab_times: 8,
        }
    }
}

impl Settings {
    /// 读取 config.ini。缺失的键回落到默认值，不会像 C# 版那样直接抛异常。
    pub fn load() -> Self {
        Self::load_from(CONFIG_FILE)
    }

    pub fn load_from(path: impl AsRef<Path>) -> Self {
        let mut ini = Ini::new();
        let _ = ini.load(path.as_ref());
        let d = Self::default();

        // C# 用 `int.Parse` / `Equals("true", OrdinalIgnoreCase)`，这里保持同样的解析口味：
        // 数字解析失败就用默认值，布尔只认 "true"（大小写不敏感）。
        let get_i32 = |key: &str, fallback: i32| {
            ini.get(SECTION, key)
                .and_then(|v| v.trim().parse::<i32>().ok())
                .unwrap_or(fallback)
        };
        let get_bool = |key: &str, fallback: bool| {
            ini.get(SECTION, key)
                .map(|v| v.eq_ignore_ascii_case("true"))
                .unwrap_or(fallback)
        };
        let get_str = |key: &str, fallback: &str| {
            ini.get(SECTION, key)
                .unwrap_or_else(|| fallback.to_string())
        };

        Self {
            version: get_str("version", &d.version),
            name: get_str("name", &d.name),
            width: get_i32("width", d.width),
            height: get_i32("height", d.height),
            max_image_count: get_i32("maximagecount", d.max_image_count),
            model_name: get_str("modelname", &d.model_name),
            is_vision_model: get_bool("isvisionmodel", d.is_vision_model),
            api_key: get_str("api_key", &d.api_key),
            server_url: get_str("server_url", &d.server_url),
            scroll: get_i32("scroll", d.scroll),
            with_image: get_bool("withimage", d.with_image),
            auto_login: get_bool("autologin", d.auto_login),
            auto_focusing: get_bool("autofocusing", d.auto_focusing),
            send_image_possibility: get_i32("sendimagepossibility", d.send_image_possibility),
            at_detect: get_bool("atdetect", d.at_detect),
            remote_server_timeout: get_i32("remote_server_timeout", d.remote_server_timeout),
            force_ollama_api: get_bool("forceollamaapi", d.force_ollama_api),
            tab_times: get_i32("tab_times", d.tab_times),
        }
    }

    /// 写回 config.ini 与 system.txt（对应 `Form1.SaveConfig`）。
    ///
    /// config.ini 会先读进来再改这几个键，所以 `scale` 等界面不管理的键、以及
    /// 文件里已有的键顺序都会被保留下来。
    pub fn save(&self, system_text: &str) -> Result<(), String> {
        self.save_to(Path::new(CONFIG_FILE), system_text, Path::new(SYSTEM_FILE))
    }

    pub fn save_to(
        &self,
        config_path: &Path,
        system_text: &str,
        system_path: &Path,
    ) -> Result<(), String> {
        let mut ini = Ini::new();
        if config_path.exists()
            && let Err(e) = ini.load(config_path)
        {
            return Err(format!("读取 {} 失败：{e}", config_path.display()));
        }

        let values: [(&str, String); 18] = [
            ("version", self.version.clone()),
            ("name", self.name.clone()),
            ("width", self.width.to_string()),
            ("height", self.height.to_string()),
            ("maximagecount", self.max_image_count.to_string()),
            ("modelname", self.model_name.clone()),
            ("isvisionmodel", bool_text(self.is_vision_model)),
            ("api_key", self.api_key.clone()),
            ("server_url", self.server_url.clone()),
            ("scroll", self.scroll.to_string()),
            ("withimage", bool_text(self.with_image)),
            ("autologin", bool_text(self.auto_login)),
            ("autofocusing", bool_text(self.auto_focusing)),
            (
                "sendimagepossibility",
                self.send_image_possibility.to_string(),
            ),
            ("atdetect", bool_text(self.at_detect)),
            (
                "remote_server_timeout",
                self.remote_server_timeout.to_string(),
            ),
            ("forceollamaapi", bool_text(self.force_ollama_api)),
            ("tab_times", self.tab_times.to_string()),
        ];
        for (key, value) in values {
            ini.set(SECTION, key, Some(value));
        }

        // 与仓库里 config.ini 的排版保持一致：`key = value`。
        let mut options = WriteOptions::default();
        options.space_around_delimiters = true;
        ini.pretty_write(config_path, &options)
            .map_err(|e| format!("写入 {} 失败：{e}", config_path.display()))?;

        fs::write(system_path, system_text)
            .map_err(|e| format!("写入 {} 失败：{e}", system_path.display()))?;

        Ok(())
    }
}

/// C# 的 `bool.ToString().ToLower()`。
fn bool_text(value: bool) -> String {
    if value { "true" } else { "false" }.to_string()
}

/// 读取 system.txt，去掉 UTF-8 BOM（C# 的 `File.ReadAllText` 会自动去掉）。
pub fn load_system_text() -> String {
    fs::read_to_string(SYSTEM_FILE)
        .map(|text| text.trim_start_matches('\u{feff}').to_string())
        .unwrap_or_default()
}

/// 对应 `TokenCount.Text = File.ReadAllText("tokencount.txt")`。
pub fn load_token_count() -> Option<String> {
    fs::read_to_string(TOKEN_FILE)
        .ok()
        .map(|text| text.trim().trim_start_matches('\u{feff}').to_string())
}

/// 对应 `ResetTokenConuter`：把计数器清成 0。
pub fn reset_token_count() -> Result<String, String> {
    fs::write(TOKEN_FILE, "0").map_err(|e| format!("写入 {TOKEN_FILE} 失败：{e}"))?;
    Ok("0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("创建临时目录失败");
        dir
    }

    #[test]
    fn round_trips_and_keeps_untouched_keys() {
        let dir = temp_dir("option3-config-roundtrip");
        let config = dir.join("config.ini");
        let system = dir.join("system.txt");
        // scale / nt_data / system 不归界面管，必须原样保留。
        fs::write(
            &config,
            "[general]\nversion = 1.5.19\nwidth = 1285\nscale = 1.75\nnt_data = None\n\
             [other]\nkeep = me\n",
        )
        .unwrap();

        let mut settings = Settings::load_from(&config);
        settings.width = 1600;
        settings.model_name = "gpt-x".to_string();
        settings.with_image = true;
        settings.save_to(&config, "系统提示", &system).unwrap();

        let written = fs::read_to_string(&config).unwrap();
        assert!(
            written.contains("scale = 1.75"),
            "scale 被弄丢了:\n{written}"
        );
        assert!(
            written.contains("nt_data = None"),
            "nt_data 被弄丢了:\n{written}"
        );
        assert!(written.contains("[other]"), "其它节被弄丢了:\n{written}");
        assert!(written.contains("keep = me"), "其它键被弄丢了:\n{written}");
        assert!(written.contains("width = 1600"));
        assert!(written.contains("withimage = true"));
        assert!(written.contains("modelname = gpt-x"));
        assert_eq!(fs::read_to_string(&system).unwrap(), "系统提示");

        let reloaded = Settings::load_from(&config);
        assert_eq!(reloaded.width, 1600);
        assert_eq!(reloaded.model_name, "gpt-x");
        assert!(reloaded.with_image);
        // 没动过的字段也要保持一致
        assert_eq!(reloaded.scroll, settings.scroll);
        assert_eq!(reloaded.version, settings.version);
    }

    #[test]
    fn missing_keys_fall_back_to_defaults() {
        let dir = temp_dir("option3-config-defaults");
        let config = dir.join("config.ini");
        fs::write(&config, "[general]\nwidth = 800\n").unwrap();

        let settings = Settings::load_from(&config);
        assert_eq!(settings.width, 800);
        assert_eq!(settings.height, Settings::default().height);
        assert_eq!(settings.tab_times, Settings::default().tab_times);
    }

    #[test]
    fn only_literal_true_counts_as_true() {
        let dir = temp_dir("option3-config-bool");
        let config = dir.join("config.ini");
        fs::write(
            &config,
            "[general]\nwithimage = yes\nautologin = 1\natdetect = True\n",
        )
        .unwrap();

        let settings = Settings::load_from(&config);
        assert!(!settings.with_image);
        assert!(!settings.auto_login);
        assert!(settings.at_detect);
    }

    #[test]
    fn missing_file_uses_defaults() {
        let dir = temp_dir("option3-config-absent");
        let settings = Settings::load_from(dir.join("nope.ini"));
        assert_eq!(settings, Settings::default());
    }
}
