//! `config.ini` 里这个工具需要的几项：窗口宽高和缩放。
//!
//! 坐标口径和 qqpilot5 一致：
//!
//! ```text
//! 实际像素 = 相对坐标 × (width × scale, height × scale)
//! ```
//!
//! 也就是把相对矩形当成"QQ 窗口内部的相对位置"，并假定窗口左上角在屏幕 (0,0)。

use std::path::Path;

use configparser::ini::Ini;

/// 配置文件里的节名。
const SECTION: &str = "general";

/// 与 qqpilot5 / option3 一致的出厂值。
const DEFAULT_WIDTH: i32 = 1285;
const DEFAULT_HEIGHT: i32 = 720;
const DEFAULT_SCALE: f64 = 1.0;

/// 界面换算要用的配置。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    /// 窗口宽度（逻辑像素）。
    pub width: i32,
    /// 窗口高度（逻辑像素）。
    pub height: i32,
    /// DPI 缩放（1.0 = 100%）。
    pub scale: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            scale: DEFAULT_SCALE,
        }
    }
}

impl Config {
    /// 读配置文件。
    ///
    /// 文件不存在、键缺失、值写坏都回落到默认值 —— 与 qqpilot5 的行为一致，
    /// 这里不报错，只是让调用方拿到的永远是一组能用的数字。
    pub fn load(path: &Path) -> Self {
        let mut ini = Ini::new();
        let _ = ini.load(path);
        let default = Self::default();

        let width = ini
            .get(SECTION, "width")
            .and_then(|value| value.trim().parse::<i32>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(default.width);
        let height = ini
            .get(SECTION, "height")
            .and_then(|value| value.trim().parse::<i32>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(default.height);
        let scale = ini
            .get(SECTION, "scale")
            .and_then(|value| value.trim().parse::<f64>().ok())
            .filter(|value| *value > 0.0)
            .unwrap_or(default.scale);

        Self {
            width,
            height,
            scale,
        }
    }

    /// 实际像素尺寸：`(width × scale, height × scale)`。
    pub fn pixel_size(&self) -> (f64, f64) {
        (
            self.width as f64 * self.scale,
            self.height as f64 * self.scale,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 测试用的临时目录：放 `target/test-tmp` 而不是 %TEMP%（有的机器不让写 %TEMP%）。
    fn temp_file(name: &str, content: &str) -> std::path::PathBuf {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn missing_file_uses_defaults() {
        let config = Config::load(Path::new("this-file-does-not-exist.ini"));
        assert_eq!(config, Config::default());
    }

    #[test]
    fn reads_width_height_and_scale() {
        let path = temp_file(
            "lr-ok.ini",
            "[general]\nwidth = 1285\nheight = 720\nscale = 1.5\n",
        );
        let config = Config::load(&path);
        assert_eq!(config.width, 1285);
        assert_eq!(config.height, 720);
        assert!((config.scale - 1.5).abs() < 1e-9);
        assert_eq!(config.pixel_size(), (1927.5, 1080.0));
    }

    #[test]
    fn broken_values_fall_back() {
        let path = temp_file(
            "lr-broken.ini",
            "[general]\nwidth = abc\nheight = -5\nscale = 0\n",
        );
        let config = Config::load(&path);
        assert_eq!(config, Config::default());
    }

    #[test]
    fn other_sections_do_not_leak_in() {
        let path = temp_file("lr-section.ini", "[other]\nwidth = 999\nscale = 2\n");
        let config = Config::load(&path);
        assert_eq!(config, Config::default());
    }
}
