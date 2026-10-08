use serde_json::Value::{self};
type Translation = Value;

use crate::log;

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

pub(crate) fn get(localization: &Translation, src: &str) -> String {
    if let Some(result) = localization.get(src)
        && let serde_json::Value::String(text) = result
    {
        return String::from(text);
    }
    log::error(format!("未发现{}的翻译。", src));
    String::from("X") + src
}

// ⚠️ 警告: DPI 感知设置失败（可能影响高分屏坐标精度）

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// 几个测试共用同一批 locale 文件，而 `fs::write` 会先截断再写，
    /// 并发跑就会读到写了一半的内容，所以用锁串起来。
    static FILE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        FILE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 中文翻译表，也就是键集合的唯一来源。
    ///
    /// 新增文案时改这张表，再跑一次测试即可生成 `zh-CN.json`。
    /// 注意这里写死的目标是 `zh-CN.json`（兜底语言），不能跟着开发机的系统语言跑，
    /// 否则在英文机器上会把中文写进 `en-US.json`。
    ///
    /// 这是不带锁的裸版本：`Mutex` 不可重入，加锁交给 [`with_default_cfg`]
    /// 和下面那个 `default_cfg` 测试，这里再拿一次会死锁。
    // fn write_default_cfg() {
    //     let json = json!({
    //             // --- 启动 / 收尾 ---
    //             "warning.dpi": "⚠️ 警告: DPI 感知设置失败（可能影响高分屏坐标精度）",
    //             "default.loading": "正在初始化",
    //             "default.loading.success": "初始化完成",
    //             "program.name": "QQPilot",
    //             "warning.minimum.bar": "请将消息栏拉到最小!",
    //             "warning.nevermovemouse": "运行时请勿移动鼠标!",
    //             "info.welcome": "欢迎您",
    //             "info.autofocus": "自动聚焦已开启",
    //             "info.autologin": "自动登录已开启",
    //             "info.login.try": "正在尝试登录",
    //             "info.quit": "退出程序",

    //             // --- 主循环 ---
    //             "info.searchingnewmessage": "正在寻找新消息",
    //             "info.found.reddot": "发现红点",
    //             "error.matchtemplate.failed": "使用模板匹配查找复制按钮失败",
    //             "warning.message.notextracted": "未提取到消息",
    //             "info.waitinganswer": "等待语言模型生成答案",
    //             "info.tokencount": "token使用总量",
    //             "warning.notgenerated": "答案未生成",
    //             "info.quitconversation": "退出会话",
    //             "info.uploadimage": "上传图片",
    //             "info.uploadgotimage": "上传收到的图片",
    //             "info.sentmessage": "发送消息",
    //             "info.sendmessage": "发消息->",
    //             "error.imagefoldernotfound": "未找到图片目录",

    //             // --- 视觉 / 模板匹配 ---
    //             "error.image.tinier": "大图尺寸小于模板图尺寸",
    //             "error.image.loadfailed": "无法加载大图",
    //             "error.template.loadfailed": "无法加载模板图",
    //             "error.unknown.code": "未知错误代码",

    //             // --- 语言模型 ---
    //             "error.answer.missing": "响应中缺少答案字段",
    //             "info.elapsed": "用时",
    //             "info.token.usage": "Token 用量",
    //             "info.token.input": "输入",
    //             "info.token.output": "输出",
    //             "info.token.total": "总计",
    //             "info.image.found": "找到",
    //             "info.image.unit": "张图片",
    //             "info.image.dataurl": "图片 DataURL",
    //             "info.image.baselength": "Base64 长度",
    //             "info.image.saved": "图片已保存到",
    //             "warning.image.notfound": "没有找到图片",
    //             "warning.image.readfailed": "读取图片失败",
    //             "error.image.badformat": "图片数据格式不正确",
    //             "error.image.unknownmime": "无法识别图片 MIME 前缀",
    //             "error.base64.decode": "base64 解码失败",
    //             "error.file.writefailed": "写入失败",
    //             "error.http.client": "创建 HTTP 客户端失败",
    //             "error.builtin.uninitialized": "内置模型尚未初始化",
    //             "error.dataset.notfound": "找不到数据集文件",
    //             "error.dataset.parsefailed": "解析数据集失败",
    //             "error.dataset.badanswer": "数据集中的答案不是字符串",
    //             "error.dataset.empty": "数据集中没有问题",

    //             // --- 聊天记录 ---
    //             "chat.self.prefix": "[你]",
    //             "chat.noimage": "无",
    //             "chat.image.label": "图片：",

    //             // --- 输入 / 剪贴板 ---
    //             "error.key.unknown": "未知键名",
    //             "error.key.modifier": "未知修饰键",
    //             "error.key.press": "未知按键",
    //             "error.clipboard.lock": "剪贴板状态锁已损坏",
    //             "error.clipboard.open": "打开剪贴板失败",
    //             "error.clipboard.write": "写入剪贴板失败",

    //             // --- 配置 ---
    //             "error.config.readfailed": "读取配置失败",
    //             "warning.config.missingkey": "缺少配置项",
    //             "warning.config.badvalue": "配置值无法解析",
    //             "warning.config.usedefault": "使用默认值",
    //             "warning.config.badjson": "配置文件顶层不是 JSON 对象",
    //             "warning.config.parsefailed": "解析配置文件失败",

    //             // --- 原生 DLL ---
    //             "error.dll.notfound": "找不到动态库",
    //             "error.dll.loadfailed": "无法加载动态库",
    //             "error.dll.symbolmissing": "缺少导出函数",

    //             // --- 加载动画 / 等待 ---
    //             "info.waiting": "等待",
    //             "info.second": "秒",
    //             "info.spinner.done": "✅ 完成！用时",
    //             "error.lock.poisoned": "状态锁已损坏",


    //                 "option.title": "设置",
    //     "option.save": "保存设置",
    //     "option.reset.token": "重置计数器",
    //     "option.extra.json": "请求的额外参数",
    //     "option.version": "版本",
    //     "option.user.name": "用户名",
    //     "option.user.name.hint": "用于判断是否是自身消息",
    //     "option.window.width": "窗口宽度",
    //     "option.window.height": "窗口高度",
    //     "option.token.count": "Token用量",
    //     "option.max.image.count": "解析图片数",
    //     "option.max.image.hint": "(本地模型解析>1张图片时速度极慢)",
    //     "option.model.name": "模型名称",
    //     "option.vision.model": "视觉模型",
    //     "option.enable.thinking": "开启思考",
    //     "option.api.key": "API Key",
    //     "option.server": "服务器",
    //     "option.force.ollama": "强制使用OllamaAPI",
    //     "option.scroll": "框选消息时长",
    //     "option.with.image": "包含图片",
    //     "option.auto.login": "自动点击登录",
    //     "option.auto.focusing": "持续将窗口置于最前",
    //     "option.send.image.possibility": "发送图片概率 (%)",
    //     "option.at.detect": "只检查 @",
    //     "option.sleep": "发送完消息后等待 (秒):",
    //     "option.timeout": "远程服务器超时 (秒):",
    //     "option.max.tokens": "最大 token 数:",
    //     "option.tab.times": "tab按下次数",
    //     "option.system.text": "提示文本",
    //     "option.server.builtin": "内置模型",
    //     "option.server.custom": "自定义",
    //     "error.config.read": "读取配置失败",
    //     "error.config.write": "写入配置失败",
    //     "error.token.reset": "重置计数器失败",
    //     "error.extra.json.open": "打开 extra.json 失败",
    //     "error.ui.init": "初始化界面失败",
    //     "error.ui.build": "构建界面失败"
    //         });
    //     std::fs::write("zh-CN.json", json.to_string()).unwrap();
    // }

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
    pub fn test() {
        with_default_cfg(|translate| {
            assert_eq!(get(translate, "default.loading"), "正在初始化");
        });
    }

    #[test]
    pub fn test_default() {
        with_default_cfg(|translate| {
            assert_eq!(get(translate, "1111111"), "X1111111");
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
        assert_eq!(get(&translate, "default.loading"), "正在初始化");
    }

    /// 空文件（建了占位但还没翻译）同样要回退，不能当成"这个语言没有文案"。
    #[test]
    fn empty_locale_file_falls_back() {
        let _guard = lock();
        write_default_cfg();
        std::fs::write("yy-YY.json", "").unwrap();
        let translate = load_for("yy-YY");
        let _ = std::fs::remove_file("yy-YY.json");
        assert_eq!(get(&translate, "default.loading"), "正在初始化");
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

        for key in ["default.loading", "info.welcome", "error.dll.notfound"] {
            assert_ne!(
                get(&translate, key),
                format!("X{key}"),
                "当前语言 {} 的表里缺 {key}",
                locale()
            );
        }
    }
}
