//! 待发送内容（对应 C# 的 `UploadContent.cs`）。
//!
//! 一次模型回复可能带三类附件：`choices[0].message.images` / `audios` / `files`。
//! 它们在这里统一收束 —— 不管来源是 base64、data URL 还是 `http(s)` 链接
//! （链接在 `answer` 里已经下载到 `./temp`），最终都变成**本地文件路径**，
//! 由 `absolute_images` / `absolute_audios` / `absolute_files` 转成绝对路径交给上传流程。

use std::path::{Path, PathBuf};

/// 一次模型回复的文本 + 需要一起发出去的图片/音频/文件。
pub struct UploadContent {
    pub text: Option<String>,
    /// `choices[0].message.images`
    pub images: Vec<String>,
    /// `choices[0].message.audios`
    pub audios: Vec<String>,
    /// `choices[0].message.files`
    pub files: Vec<String>,
}

impl UploadContent {
    pub fn new(
        text: Option<String>,
        images: Vec<String>,
        audios: Vec<String>,
        files: Vec<String>,
    ) -> Self {
        Self {
            text,
            images,
            audios,
            files,
        }
    }

    /// 只要文本（对应 C# 里 `UploadContent(text)` 那个重载）。
    pub fn text_only(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            images: Vec::new(),
            audios: Vec::new(),
            files: Vec::new(),
        }
    }

    /// 对应 `UploadContent.Absolute()`：把图片路径转成绝对路径。
    ///
    /// C# 用的是 `Path.GetFullPath`，这里用 `std::path::absolute`（同样不改动路径写法，
    /// 不会像 `canonicalize` 那样加上 `\\?\` 前缀而破坏文件选择对话框）。
    pub fn absolute_images(&self) -> Vec<String> {
        absolute_all(&self.images)
    }

    /// 音频的绝对路径（规则同 [`Self::absolute_images`]）。
    pub fn absolute_audios(&self) -> Vec<String> {
        absolute_all(&self.audios)
    }

    /// 文件的绝对路径（规则同 [`Self::absolute_images`]）。
    pub fn absolute_files(&self) -> Vec<String> {
        absolute_all(&self.files)
    }

    /// 有没有任何附件。
    pub fn has_attachments(&self) -> bool {
        !self.images.is_empty() || !self.audios.is_empty() || !self.files.is_empty()
    }
}

/// 把一批路径转成绝对路径；解析失败就原样返回。
fn absolute_all(paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .map(|path| {
            std::path::absolute(Path::new(path))
                .unwrap_or_else(|_| PathBuf::from(path))
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_only_has_no_attachments() {
        let content = UploadContent::text_only("hi");
        assert_eq!(content.text.as_deref(), Some("hi"));
        assert!(!content.has_attachments());
    }

    #[test]
    fn every_kind_gets_absolute_paths() {
        let content = UploadContent::new(
            None,
            vec![String::from("a.png")],
            vec![String::from("b.mp3")],
            vec![String::from("c.pdf")],
        );
        assert!(content.has_attachments());
        assert!(content.absolute_audios()[0].ends_with("b.mp3"));
        assert!(content.absolute_files()[0].ends_with("c.pdf"));
    }

    #[test]
    fn absolute_paths_are_resolved() {
        let content = UploadContent::new(None, vec![String::from("temp/x.png")], vec![], vec![]);
        let resolved = content.absolute_images();
        assert!(Path::new(&resolved[0]).is_absolute(), "{}", resolved[0]);
    }
}
