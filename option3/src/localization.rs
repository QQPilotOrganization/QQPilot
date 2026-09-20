//! 界面文案翻译表。
//!
//! 与 qqpilot5 的 `localization.rs` 同一套用法（`load()` / `get()`），
//! 额外加了一个 [`text`]，因为 native-windows-gui 的 `text:` 属性是编译期常量，
//! 只能吃 `&'static str`，而翻译是运行期从 `localization.json` 读的。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::Value::{self};
type Translation = Value;

const DATA1: &str = r#"{}"#;

/// 读取 `localization.json`。
pub(crate) fn load() -> Translation {
    let data = std::fs::read_to_string("localization.json").unwrap_or_default();
    let v: Translation =
        serde_json::from_str(&data).unwrap_or(serde_json::from_str(DATA1).unwrap());
    v
}

/// 取一条翻译。找不到时提示并返回 `X<key>`，方便一眼看出漏了哪条。
pub(crate) fn get(localization: &Translation, src: &str) -> String {
    if let Some(Value::String(text)) = localization.get(src) {
        return text.clone();
    }
    eprintln!("未发现{src}的翻译。");
    format!("X{src}")
}

/// 把翻译固化成立即 `&'static str`，供 `#[nwg_control(text: ...)]` 使用。
///
/// 每个 key 只在第一次用到时泄漏一次，key 的数量等于控件数，固定不再增长。
pub(crate) fn text(key: &str) -> &'static str {
    static CACHE: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    // 锁中毒也不用慌，这里只是缓存，`into_inner` 继续用。
    let mut guard = cache.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(value) = guard.get(key) {
        return value;
    }
    let leaked: &'static str = Box::leak(get(&load(), key).into_boxed_str());
    guard.insert(key.to_string(), leaked);
    leaked
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// 全部翻译键。这个测试会把它们写进 `localization.json`，
    /// 所以它是键集合的唯一来源：新增文案时改这里，再跑一次测试即可生成文件。
    #[test]
    pub fn default_cfg() {
        let json = json!({
            // --- 窗口 / 按钮 ---
            "option.title": "设置",
            "option.save": "保存设置",
            "option.reset.token": "重置计数器",
            "option.extra.json": "请求的额外参数",

            // --- 标签 ---
            "option.version": "版本",
            "option.user.name": "用户名",
            "option.user.name.hint": "用于判断是否是自身消息",
            "option.window.width": "窗口宽度",
            "option.window.height": "窗口高度",
            "option.token.count": "Token用量",
            "option.max.image.count": "解析图片数",
            "option.max.image.hint": "(本地模型解析>1张图片时速度极慢)",
            "option.model.name": "模型名称",
            "option.vision.model": "视觉模型",
            "option.api.key": "API Key",
            "option.server": "服务器",
            "option.force.ollama": "强制使用OllamaAPI",
            "option.scroll": "框选消息时长",
            "option.with.image": "包含图片",
            "option.auto.login": "自动点击登录",
            "option.auto.focusing": "持续将窗口置于最前",
            "option.send.image.possibility": "发送图片概率 (%)",
            "option.at.detect": "只检查 @",
            "option.sleep": "发送完消息后等待 (秒):",
            "option.timeout": "远程服务器超时 (秒):",
            "option.tab.times": "tab按下次数",
            "option.system.text": "提示文本",

            // --- 服务器下拉（第一项 "ollama" 是协议值，不做翻译）---
            "option.server.builtin": "内置模型",
            "option.server.custom": "自定义",

            // --- 错误提示 ---
            "error.config.read": "读取配置失败",
            "error.config.write": "写入配置失败",
            "error.token.reset": "重置计数器失败",
            "error.extra.json.open": "打开 extra.json 失败",
            "error.ui.init": "初始化界面失败",
            "error.ui.build": "构建界面失败"
        });

        std::fs::write("localization.json", json.to_string()).unwrap();
    }

    /// 生成 translation 表并跑一段断言。
    ///
    /// 几个测试共用 `localization.json` 这一个文件，而 `fs::write` 会先截断再写，
    /// 并发跑就会读到写了一半的内容，所以这里用锁串起来。
    fn with_default_cfg<T>(f: impl FnOnce(&Translation) -> T) -> T {
        static LOCK: Mutex<()> = Mutex::new(());
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        default_cfg();
        f(&load())
    }

    #[test]
    pub fn loads_expected_value() {
        with_default_cfg(|translate| assert_eq!(get(translate, "option.title"), "设置"));
    }

    #[test]
    pub fn missing_key_is_marked() {
        with_default_cfg(|translate| {
            assert_eq!(get(translate, "no.such.key"), "Xno.such.key");
        });
    }

    #[test]
    pub fn text_is_stable_and_cached() {
        with_default_cfg(|_| {
            let first = text("option.title");
            let second = text("option.title");
            assert_eq!(first, "设置");
            // 同一个 key 复用同一块内存，不会反复泄漏
            assert!(std::ptr::eq(first, second));
        });
    }
}
