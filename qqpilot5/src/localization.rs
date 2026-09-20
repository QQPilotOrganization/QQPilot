use serde_json::Value::{self};
type Translation = Value;

use crate::log;

const DATA1: &str = r#"{}"#;
pub(crate) fn load() -> Translation {
    let data = std::fs::read_to_string("localization.json").unwrap_or(String::new());
    let v: Translation =
        serde_json::from_str(&data).unwrap_or(serde_json::from_str(DATA1).unwrap());
    v
}
pub(crate) fn get(localization: &Translation, src: &str) -> String {
    if let Some(result) = localization.get(src) {
        if let serde_json::Value::String(text) = result {
            return String::from(text);
        }
    }
    log::error(format!("未发现{}的翻译。", src));
    String::from(String::from("X") + &src)
}

// ⚠️ 警告: DPI 感知设置失败（可能影响高分屏坐标精度）

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    pub fn default_cfg() {
        let json = json!({
             "warning.dpi":"⚠️ 警告: DPI 感知设置失败（可能影响高分屏坐标精度）",
             "default.loading":"正在初始化",
             "default.loading.success":"初始化完成",
             "program.name":"QQPilot",
             "warning.minimum.bar":"请将消息栏拉到最小!",
             "info.found.reddot":"发现红点",
             "warning.nevermovemouse":"运行时请勿移动鼠标!",
             "info.autofocus":"自动聚焦已开启",
             "info.autologin":"自动登录已开启",
             "info.login.try":"正在尝试登录",
             "info.searchingnewmessage":"正在寻找新消息",
            "info.found.reddot":"发现红点",
            "error.matchtemplate.failed":"使用模板匹配查找复制按钮失败",
            "warning.message.notextracted":"未提取到消息",
            "info.waitinganswer":"等待语言模型生成答案",
            "info.tokencount":"token使用总量",
            "warning.notgenerated":"答案未生成",
            "info.quitconversation":"退出会话",
            "info.uploadimage":"上传图片",
             "info.uploadgotimage":"上传收到的图片",
             "info.sentmessage":"发送消息",
             "info.quitconversation":"退出会话",
             "info.quit":"退出程序",
             "error.imagefoldernotfound":"未找到图片目录",
             "info.welcome":"欢迎您"
        });

        std::fs::write("localization.json", json.to_string()).unwrap();
        assert!(true);
    }
    #[test]
    pub fn test() {
        default_cfg();
        let x = load();
        assert_eq!(get(&x, "default.loading"), "正在初始化");
    }

    #[test]
    pub fn test_default() {
        default_cfg();
        let x = load();
        assert_eq!(get(&x, "1111111"), "X1111111");
    }
}
