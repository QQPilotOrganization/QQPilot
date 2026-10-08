//! 界面文案翻译表。
//!
//! 与 qqpilot5 的 `localization.rs` 同一套用法（`load()` / `get()`），
//! 额外加了一个 [`text`]，因为 native-windows-gui 的 `text:` 属性是编译期常量，
//! 只能吃 `&'static str`，而翻译是运行期从 `<语言>.json` 读的。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use serde_json::Value::{self};
type Translation = Value;

/// 支持的语言。**第一项是兜底语言**，任何找不到的文案都从这里取。
pub(crate) const LOCALES: [&str; 2] = ["zh-CN", "en-US"];
const FALLBACK_LOCALE: &str = "zh-CN";

/// 当前系统语言对应的翻译。
///
/// 系统语言不在 [`LOCALES`] 里时返回兜底语言，所以调用方拿到的永远是受支持的 key。
pub(crate) fn locale() -> &'static str {
    let current = current_locale::current_locale().unwrap_or_default();
    LOCALES
        .iter()
        .copied()
        .find(|name| name.eq_ignore_ascii_case(&current))
        .unwrap_or(FALLBACK_LOCALE)
}

/// 读某个语言的翻译文件。
///
/// 文件缺失、内容为空、或者顶层不是对象，都返回 `None` —— 这样"建了个空文件占位"
/// 也会正确地回退到兜底语言，而不是让整份界面变成 `X<key>`。
fn read_locale(name: &str) -> Option<Translation> {
    let data = std::fs::read_to_string(format!("{name}.json")).ok()?;
    match serde_json::from_str::<Translation>(&data) {
        Ok(Value::Object(map)) if !map.is_empty() => Some(Value::Object(map)),
        _ => None,
    }
}

/// 按 `当前语言 -> 兜底语言` 的顺序加载。
pub(crate) fn load() -> Translation {
    load_for(locale())
}

fn load_for(name: &str) -> Translation {
    read_locale(name)
        .or_else(|| read_locale(FALLBACK_LOCALE))
        .unwrap_or_else(|| serde_json::from_str(r#"{}"#).unwrap())
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

    /// 几个测试共用同一批 locale 文件，而 `fs::write` 会先截断再写，
    /// 并发跑就会读到写了一半的内容，所以用锁串起来。
    static FILE_LOCK: Mutex<()> = Mutex::new(());

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        FILE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 用qqpilot5的那个
    fn write_default_cfg() {


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
            "option.enable.thinking": "开启思考",
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
            "option.max.tokens": "最大 token 数:",
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

        std::fs::write("zh-CN.json", json.to_string()).unwrap();
    }

    /// 把中文表写出来（这是生成 `zh-CN.json` 的入口）。
    #[test]
    pub fn default_cfg() {
        let _guard = lock();
        write_default_cfg();
    }

    /// 生成中文表并跑一段断言。
    ///
    /// 交给闭包的是**兜底语言**的表，而不是 `load()`：`load()` 会跟着开发机的
    /// 系统语言走，断言具体文案的测试在英文机器上就会挂。
    fn with_default_cfg<T>(f: impl FnOnce(&Translation) -> T) -> T {
        let _guard = lock();
        write_default_cfg();
        f(&load_for(FALLBACK_LOCALE))
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
            // 值跟着当前系统语言走，但绝不能是缺翻译的兜底值
            assert_ne!(first, "Xoption.title");
            assert_eq!(first, get(&load(), "option.title"));
            // 同一个 key 复用同一块内存，不会反复泄漏
            assert!(std::ptr::eq(first, second));
        });
    }

    /// 每个受支持语言的文件都要存在，而且键集合必须和兜底语言完全一致 ——
    /// 少一条就会在界面上显示成 `X<key>`，多一条则说明漏了代码引用。
    #[test]
    fn every_locale_has_the_same_keys() {
        let _guard = lock();
        write_default_cfg();

        let reference: Vec<String> = match read_locale(FALLBACK_LOCALE) {
            Some(Value::Object(map)) => map.keys().cloned().collect(),
            other => panic!("{FALLBACK_LOCALE}.json 不可用: {other:?}"),
        };
        assert!(!reference.is_empty(), "兜底语言表是空的");

        for name in LOCALES {
            let Some(Value::Object(map)) = read_locale(name) else {
                panic!("缺少 {name}.json（或它是空的）");
            };
            let keys: Vec<String> = map.keys().cloned().collect();

            let missing: Vec<&String> = reference.iter().filter(|k| !keys.contains(k)).collect();
            let extra: Vec<&String> = keys.iter().filter(|k| !reference.contains(k)).collect();
            assert!(missing.is_empty(), "{name}.json 少了这些键: {missing:?}");
            assert!(extra.is_empty(), "{name}.json 多了这些键: {extra:?}");
        }
    }

    /// 系统语言没被支持（或对应文件是空的）时，必须回退到兜底语言，
    /// 而不是返回一张空表让整份界面变成 `X<key>`。
    #[test]
    fn unsupported_locale_falls_back() {
        let _guard = lock();
        write_default_cfg();
        let translate = load_for("xx-XX");
        assert_eq!(get(&translate, "option.title"), "设置");
    }

    /// 空文件（建了占位但还没翻译）同样要回退，不能当成"这个语言没有文案"。
    #[test]
    fn empty_locale_file_falls_back() {
        let _guard = lock();
        write_default_cfg();
        std::fs::write("yy-YY.json", "").unwrap();
        let translate = load_for("yy-YY");
        let _ = std::fs::remove_file("yy-YY.json");
        assert_eq!(get(&translate, "option.title"), "设置");
    }

    /// `locale()` 必须落在支持列表里，否则拼出来的文件名读不到东西。
    #[test]
    fn locale_is_always_supported() {
        assert!(LOCALES.contains(&locale()));
    }

    /// `load()` 要按当前系统语言给出完整的表。
    ///
    /// 不管最后落到哪个语言，都不能出现 `get` 的兜底值 `X<key>` ——
    /// 那正是"机器语言没有对应文件"时会看到的症状。
    #[test]
    fn load_serves_a_complete_table_for_the_current_locale() {
        let _guard = lock();
        write_default_cfg();
        let translate = load();

        for key in ["option.title", "option.save", "error.ui.init"] {
            assert_ne!(
                get(&translate, key),
                format!("X{key}"),
                "当前语言 {} 的表里缺 {key}",
                locale()
            );
        }
    }
}
