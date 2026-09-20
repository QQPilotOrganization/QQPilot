use serde_json::Value::{self};
type Translation = Value;

use crate::log;

const DATA1: &str = r#"{}"#;
pub(crate) fn load() -> Translation {
    let data = std::fs::read_to_string("localization.json").unwrap_or_default();
    let v: Translation =
        serde_json::from_str(&data).unwrap_or(serde_json::from_str(DATA1).unwrap());
    v
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

    /// 全部翻译键。这个测试会把它们写进 `localization.json`，
    /// 所以它是键集合的唯一来源：新增文案时改这里，再跑一次测试即可生成文件。
    #[test]
    pub fn default_cfg() {
        let json = json!({
            // --- 启动 / 收尾 ---
            "warning.dpi": "⚠️ 警告: DPI 感知设置失败（可能影响高分屏坐标精度）",
            "default.loading": "正在初始化",
            "default.loading.success": "初始化完成",
            "program.name": "QQPilot",
            "warning.minimum.bar": "请将消息栏拉到最小!",
            "warning.nevermovemouse": "运行时请勿移动鼠标!",
            "info.welcome": "欢迎您",
            "info.autofocus": "自动聚焦已开启",
            "info.autologin": "自动登录已开启",
            "info.login.try": "正在尝试登录",
            "info.quit": "退出程序",

            // --- 主循环 ---
            "info.searchingnewmessage": "正在寻找新消息",
            "info.found.reddot": "发现红点",
            "error.matchtemplate.failed": "使用模板匹配查找复制按钮失败",
            "warning.message.notextracted": "未提取到消息",
            "info.waitinganswer": "等待语言模型生成答案",
            "info.tokencount": "token使用总量",
            "warning.notgenerated": "答案未生成",
            "info.quitconversation": "退出会话",
            "info.uploadimage": "上传图片",
            "info.uploadgotimage": "上传收到的图片",
            "info.sentmessage": "发送消息",
            "info.sendmessage": "发消息->",
            "error.imagefoldernotfound": "未找到图片目录",

            // --- 视觉 / 模板匹配 ---
            "error.image.tinier": "大图尺寸小于模板图尺寸",
            "error.image.loadfailed": "无法加载大图",
            "error.template.loadfailed": "无法加载模板图",
            "error.unknown.code": "未知错误代码",

            // --- 语言模型 ---
            "error.answer.missing": "响应中缺少答案字段",
            "info.elapsed": "用时",
            "info.token.usage": "Token 用量",
            "info.token.input": "输入",
            "info.token.output": "输出",
            "info.token.total": "总计",
            "info.image.found": "找到",
            "info.image.unit": "张图片",
            "info.image.dataurl": "图片 DataURL",
            "info.image.baselength": "Base64 长度",
            "info.image.saved": "图片已保存到",
            "warning.image.notfound": "没有找到图片",
            "warning.image.readfailed": "读取图片失败",
            "error.image.badformat": "图片数据格式不正确",
            "error.image.unknownmime": "无法识别图片 MIME 前缀",
            "error.base64.decode": "base64 解码失败",
            "error.file.writefailed": "写入失败",
            "error.http.client": "创建 HTTP 客户端失败",
            "error.builtin.uninitialized": "内置模型尚未初始化",
            "error.dataset.notfound": "找不到数据集文件",
            "error.dataset.parsefailed": "解析数据集失败",
            "error.dataset.badanswer": "数据集中的答案不是字符串",
            "error.dataset.empty": "数据集中没有问题",

            // --- 聊天记录 ---
            "chat.self.prefix": "[你]",
            "chat.noimage": "无",
            "chat.image.label": "图片：",

            // --- 输入 / 剪贴板 ---
            "error.key.unknown": "未知键名",
            "error.key.modifier": "未知修饰键",
            "error.key.press": "未知按键",
            "error.clipboard.lock": "剪贴板状态锁已损坏",
            "error.clipboard.open": "打开剪贴板失败",
            "error.clipboard.write": "写入剪贴板失败",

            // --- 配置 ---
            "error.config.readfailed": "读取配置失败",
            "warning.config.missingkey": "缺少配置项",
            "warning.config.badvalue": "配置值无法解析",
            "warning.config.usedefault": "使用默认值",
            "warning.config.badjson": "配置文件顶层不是 JSON 对象",
            "warning.config.parsefailed": "解析配置文件失败",

            // --- 原生 DLL ---
            "error.dll.notfound": "找不到动态库",
            "error.dll.loadfailed": "无法加载动态库",
            "error.dll.symbolmissing": "缺少导出函数",

            // --- 加载动画 / 等待 ---
            "info.waiting": "等待",
            "info.second": "秒",
            "info.spinner.done": "✅ 完成！用时",
            "error.lock.poisoned": "状态锁已损坏"
        });

        std::fs::write("localization.json", json.to_string()).unwrap();
    }
    /// 生成 translation 表并跑一段断言。
    ///
    /// 几个测试共用 `localization.json` 这一个文件，而 `fs::write` 会先截断再写，
    /// 并发跑就会读到写了一半的内容，所以这里用锁串起来。
    fn with_default_cfg<T>(f: impl FnOnce(&Translation) -> T) -> T {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
        default_cfg();
        f(&load())
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
}
