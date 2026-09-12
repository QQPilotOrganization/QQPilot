//! 聊天记录解析（对应 C# 的 `ConversationStyleExtract.cs`）。

use std::path::MAIN_SEPARATOR;
use std::sync::OnceLock;

use regex::Regex;

use crate::chat_content::ChatContent;
use crate::log;

/// 形如 `用户名: 08-10 17:18:30` 的消息头。
const HEADER_PATTERN: &str = r"^(.+?):\s+(\d{2}-\d{2}\s+\d{2}:\d{2}:\d{2})$";
/// 形如 `<img src="file://..." />` 的图片标签。
const IMG_PATTERN: &str = r#"<img\s+[^>]*?src\s*=\s*['"]([^'"]+)['"][^>]*>"#;

fn header_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(HEADER_PATTERN).expect("消息头正则表达式无效"))
}

fn image_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(&format!("(?i){IMG_PATTERN}")).expect("图片标签正则表达式无效"))
}

/// 对应 `ConversationStyleExtract.ExtractImagePaths`：
/// 取出所有 `<img src=...>` 的本地路径，并返回去掉标签后的纯文本。
pub fn extract_image_paths(text: &str) -> (Vec<String>, String) {
    let regex = image_regex();
    let mut image_paths = Vec::new();

    for captures in regex.captures_iter(text) {
        if let Some(src) = captures.get(1) {
            let path = process_path(src.as_str());
            if !path.is_empty() {
                image_paths.push(path);
            }
        }
    }

    let clean_text = regex.replace_all(text, "").trim().to_string();
    (image_paths, clean_text)
}

/// 对应 `ConversationStyleExtract.ProcessPath`：去掉 `file://`、修复 `/D:/`、URL 解码、统一分隔符。
fn process_path(src: &str) -> String {
    let mut path = src.to_string();

    // 1. 去掉 file:// 协议头
    if path.len() >= 7 && path.as_bytes()[..7].eq_ignore_ascii_case(b"file://") {
        path = path[7..].to_string();
    }

    // 2. Windows 路径修复：/D:/... -> D:/...
    if path.starts_with('/') && path.len() >= 3 && path.as_bytes()[2] == b':' {
        path = path[1..].to_string();
    }

    // 3. URL 解码（%20 之类）
    path = percent_encoding::percent_decode_str(&path)
        .decode_utf8_lossy()
        .into_owned();

    // 4. 统一分隔符
    let separator = MAIN_SEPARATOR.to_string();
    path.replace(['/', '\\'], &separator)
}

/// 对应 C# 的 `Split(["\r\n", "\r", "\n"], StringSplitOptions.None)`。
///
/// 刻意不用 `str::lines()`：`lines()` 不把单独的 `\r` 当作换行，而 C# 会。
fn split_lines(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut index = 0;

    while index < bytes.len() {
        match bytes[index] {
            b'\r' => {
                lines.push(&text[start..index]);
                index += if bytes.get(index + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
                start = index;
            }
            b'\n' => {
                lines.push(&text[start..index]);
                index += 1;
                start = index;
            }
            _ => index += 1,
        }
    }
    lines.push(&text[start..]);
    lines
}

/// 对应 `ConversationStyleExtract.ParseChatLog`。
pub fn parse_chat_log(chat: &str, character_name: &str) -> Vec<ChatContent> {
    let header = header_regex();
    let lines = split_lines(chat);
    let mut messages = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim_end();
        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        let Some(captures) = header.captures(line) else {
            i += 1;
            continue;
        };

        let username = captures[1].to_string();
        let time = captures[2].to_string();
        i += 1;
        let mut content_lines: Vec<&str> = Vec::new();

        while i < lines.len() {
            let next_line = lines[i].trim_end();
            if next_line.trim().is_empty() {
                i += 1;
                continue;
            }
            if header.is_match(next_line) {
                break;
            }
            content_lines.push(lines[i]);
            i += 1;
        }

        let raw_text = content_lines.join("\n");
        let (image_paths, clean_text) = extract_image_paths(&raw_text);
        let own_by_myself = username.trim() == character_name.trim();

        messages.push(ChatContent::new(
            username,
            image_paths,
            clean_text,
            time,
            own_by_myself,
        ));

        log::print(messages.last().map(|m| m.report()).unwrap_or_default());
        log::print("");
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 取自 C# `ConversationStyleExtract.Test()` 的样例。
    const SAMPLE: &str = "neko: 08-10 17:18:30\r\n这就是……我的……猫爪攻击喵~\r\n\r\nneko: 08-10 17:18:31\r\n喵～网易监管确实像摆设呢\r\n\r\nneko: 08-10 17:18:46\r\n<img src=\"file://E:\\tmp\\nt_data\\Pic\\ea3229f89fee9510a42a294f7f40ae6c_720.jpg\" />\r\n\r\nOurMovement: 08-10 17:19:55\r\n<img src=\"file://C:\\Users\\Dev\\Desktop\\93db.jpg \" />\r\n\r\nOurMovement: 08-10 17:19:57\r\n2￥\r\n\r\nneko: 08-10 17:20:54\r\n什么发言这么唐啊喵~\r\n\r\n蓝色鲸鱼: 08-10 17:21:11\r\n又发图喵？\r\n\r\n话说你们说本喵被neko教坏了\r\n\r\nbaiyan: 08-10 17:23:19\r\n试一下\r\n";

    #[test]
    fn splits_lines_like_dotnet() {
        assert_eq!(split_lines("a\r\nb"), vec!["a", "b"]);
        assert_eq!(split_lines("a\rb"), vec!["a", "b"]);
        assert_eq!(split_lines("a\nb"), vec!["a", "b"]);
        assert_eq!(split_lines("a\r\n"), vec!["a", ""]);
        assert_eq!(split_lines(""), vec![""]);
    }

    #[test]
    fn extracts_image_paths_and_strips_tags() {
        let (paths, text) =
            extract_image_paths("看图<img src=\"file://E:\\tmp\\a%20b.png\" />好玩吗");
        assert_eq!(paths, vec!["E:\\tmp\\a b.png".to_string()]);
        assert_eq!(text, "看图好玩吗");
    }

    #[test]
    fn keeps_path_whitespace_inside_quotes() {
        // C# 不做 trim，末尾空格会留在路径里。
        let (paths, _) = extract_image_paths("<img src=\"file://C:\\a.jpg \" />");
        assert_eq!(paths, vec!["C:\\a.jpg ".to_string()]);
    }

    #[test]
    fn parses_sample_chat_log() {
        let messages = parse_chat_log(SAMPLE, "neko");
        assert_eq!(messages.len(), 8);

        assert_eq!(messages[0].username, "neko");
        assert_eq!(messages[0].time, "08-10 17:18:30");
        assert!(messages[0].own_by_myself);

        // 第三条只有图片，纯文本应为空
        assert_eq!(messages[2].text, "");
        assert_eq!(messages[2].image_paths.len(), 1);

        // 对方发来的图片
        assert!(!messages[3].own_by_myself);
        assert_eq!(messages[3].image_paths.len(), 1);

        // 多行消息：空行会被跳过，剩下的用 \n 连接
        assert_eq!(messages[6].username, "蓝色鲸鱼");
        assert_eq!(messages[6].text, "又发图喵？\n话说你们说本喵被neko教坏了");
    }

    #[test]
    fn character_name_is_trimmed_for_ownership() {
        let messages = parse_chat_log("neko: 08-10 17:18:30\r\nhi\r\n", " neko ");
        assert!(messages[0].own_by_myself);
    }
}
