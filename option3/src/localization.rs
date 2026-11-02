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
///
/// 现在只有中文一份表（英文表已经删了）；以后要加语言，把名字加进来、
/// 再放一份同名的 `<locale>.json` 即可，`load()` 会自动按系统语言选。
pub(crate) const LOCALES: [&str; 1] = ["zh-CN"];
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
    use super::*;

    /// 几个测试共用同一批 locale 文件，而 `fs::write` 会先截断再写，
    /// 并发跑就会读到写了一半的内容，所以用锁串起来。
    static FILE_LOCK: Mutex<()> = Mutex::new(());

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        FILE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 中文表：`zh-CN.json` 的唯一来源。
    ///
    /// 用 `from_str` 而不是 `json!` 宏 —— 键多了以后宏的递归深度会撞上默认上限，
    /// 而且这样和 qqpilot5 的写法一致，增删一条只会动一行 diff。
    fn write_default_cfg() {
        let json: Translation = serde_json::from_str(
            r#"{
    "option.title": "设置",
    "option.save": "保存设置",
    "option.reset.token": "重置计数器",
    "option.extra.json": "请求的额外参数",
    "option.run_connect_test": "连通性测试",
    "option.testconnect.testing": "测试中…",
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
    "option.max_output": "最大输出字数",
    "option.rest.schedule": "休息时间表",
    "option.rest.schedule.hint": "在设定时间内不工作,以本地时间小时为单位,用英文逗号分隔,例如:1,2,3,4,5;None为不启用",
    "option.simulate.output.speed": "模拟输出速度",
    "option.tab.after.match.template.failed": "模板匹配失败后使用tab选择复制按钮",
    "option.tab.times": "tab按下次数",
    "option.system.text": "提示文本",

    "option.server.builtin": "内置模型",
    "option.server.custom": "自定义",
    "option.mouse.smooth": "平滑鼠标移动",

    "option.group.basic": "基本",
    "option.group.model": "模型与接口",
    "option.group.media": "图片与消息",
    "option.group.rhythm": "节奏与输出",
    "option.group.system": "提示文本",
    "option.group.plan": "运行计划",
    "option.group.auto": "自动化",
    "option.group.status": "状态",

    "option.hint.window.size": "设置QQ窗口的宽高",
    "option.hint.model": "例如 Qwen/Qwen3-8B",
    "option.hint.api.key": "sk-****",
    "option.hint.server": "https://example.com/v1",
    "option.hint.timeout": "等服务器响应的最长时间",
    "option.hint.system.text": "模型提示词",
    "option.warning.testconnect.consume": "向填写的服务器发送一次请求，会消耗token，是否继续？",
    "option.testconnect.success.title": "连接成功",
    "option.testconnect.failed.title": "连接失败",
    "option.testconnect.builtin": "内置模型不联网，不需要测试连接。",
    "option.testconnect.url": "地址",
    "option.testconnect.reply": "答复",
    "option.testconnect.timeout": "请求超时",
    "option.testconnect.notjson": "响应不是合法的 JSON（地址可能不是 API 根路径）",
    
    "error.config.read": "读取配置失败",
    "error.config.write": "写入配置失败",
    "error.token.reset": "重置计数器失败",
    "error.extra.json.open": "打开 extra.json 失败",
    "error.ui.init": "初始化界面失败",
    "error.ui.build": "构建界面失败"
}"#,
        )
        .unwrap();

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

    /// 兜底语言表要读得出来、不能为空、也不能有条目是空文案。
    ///
    /// 以前这里还检查"每个受支持语言的键集合都要一致"；英文表删掉以后只剩一份表，
    /// 所以改成检查这一份的质量。以后再加语言时，把逐语言对比补回来即可。
    #[test]
    fn fallback_table_is_usable() {
        let _guard = lock();
        write_default_cfg();

        let Some(Value::Object(map)) = read_locale(FALLBACK_LOCALE) else {
            panic!("读不到 {FALLBACK_LOCALE}.json");
        };
        assert!(!map.is_empty(), "兜底语言表是空的");
        for (key, value) in &map {
            let text = value
                .as_str()
                .unwrap_or_else(|| panic!("{key} 的文案不是字符串"));
            assert!(!text.trim().is_empty(), "{key} 的文案是空的");
        }

        // 系统语言不认识时也要拿到同一份表，不能让界面变成 `X<key>`。
        assert_eq!(load_for("xx-XX"), Value::Object(map));
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
