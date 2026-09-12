//! 待发送内容（对应 C# 的 `UploadContent.cs`）。

use std::path::{Path, PathBuf};

/// 一次模型回复的文本 + 需要一起发出去的图片。
pub struct UploadContent {
    pub text: Option<String>,
    pub images: Vec<String>,
}

impl UploadContent {
    pub fn new(text: Option<String>, images: Vec<String>) -> Self {
        Self { text, images }
    }

    /// 只要文本（对应 C# 里 `UploadContent(text)` 那个重载）。
    pub fn text_only(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            images: Vec::new(),
        }
    }

    /// 对应 `UploadContent.Absolute()`：把图片路径转成绝对路径。
    ///
    /// C# 用的是 `Path.GetFullPath`，这里用 `std::path::absolute`（同样不改动路径写法，
    /// 不会像 `canonicalize` 那样加上 `\\?\` 前缀而破坏文件选择对话框）。
    pub fn absolute(&self) -> Vec<String> {
        self.images
            .iter()
            .map(|image| {
                std::path::absolute(Path::new(image))
                    .unwrap_or_else(|_| PathBuf::from(image))
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }
}
