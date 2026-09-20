//! 单条聊天消息（对应 C# 的 `ChatContent.cs`）。

use std::fmt;
use std::path::Path;

use chrono::Local;

use crate::localization;

/// 一条从聊天记录里解析出来的消息。
pub struct ChatContent {
    pub username: String,
    pub image_paths: Vec<String>,
    pub text: String,
    pub time: String,
    /// 是否是自己发出的消息（决定它在请求里是 assistant 还是 user）。
    pub own_by_myself: bool,
}

impl ChatContent {
    pub fn new(
        username: impl Into<String>,
        image_paths: Vec<String>,
        text: impl Into<String>,
        time: impl Into<String>,
        own_by_myself: bool,
    ) -> Self {
        Self {
            username: username.into(),
            image_paths,
            text: text.into(),
            time: time.into(),
            own_by_myself,
        }
    }

    /// 对应 `ChatContent.Report()`：带图片存在性检查的可读摘要。
    pub fn report(&self) -> String {
        let translate = localization::load();
        let t = |key: &str| localization::get(&translate, key);

        let prefix = if self.own_by_myself {
            t("chat.self.prefix")
        } else {
            String::new()
        };
        let content = &self.text;

        let valid_images: Vec<&String> = self
            .image_paths
            .iter()
            .filter(|path| Path::new(path.as_str()).exists())
            .collect();

        let image_part = if valid_images.is_empty() {
            t("chat.noimage")
        } else {
            let joined = valid_images
                .iter()
                .map(|p| format!("\"{p}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!("[ {joined} ]")
        };

        format!(
            "{prefix}{username}: {content}\n{time}\n {label}{image_part}",
            username = self.username,
            time = self.time,
            label = t("chat.image.label"),
        )
    }
}

/// C# 的 `content[..^1]`：去掉最后一个字符（C# 按 UTF-16 单元，这里按 Unicode 标量）。
fn drop_last_char(text: &str) -> &str {
    match text.char_indices().next_back() {
        Some((index, _)) => &text[..index],
        None => text,
    }
}

impl fmt::Display for ChatContent {
    /// 对应 `ChatContent.ToString()`：拼成发给语言模型的文本片段。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.own_by_myself {
            if self.text.is_empty() {
                return Ok(());
            }
            return f.write_str(drop_last_char(&self.text));
        }

        let time = if self.time.is_empty() {
            Local::now().format("%m-%d %H:%M:%S").to_string()
        } else {
            self.time.clone()
        };

        write!(
            f,
            "[time]\n{time} \n\n [username] \n {} \n\n [content] \n {}\n",
            self.username, self.text
        )
    }
}
