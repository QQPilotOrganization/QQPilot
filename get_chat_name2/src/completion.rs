use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub(crate) struct ChatRequest {
    pub(crate) model: String,
    pub(crate) messages: Vec<Message>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Message {
    pub(crate) role: String,
    pub(crate) content: Vec<Content>,
}

/// OpenAI 兼容的多模态 content part。
/// 序列化结果：`{"type":"text","text":"OCR:"}` / `{"type":"image_url","image_url":{"url":"..."}}`
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum Content {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ImageUrl {
    pub(crate) url: String,
}

// 用于接收返回的 JSON 结构（choices[].message.content 是字符串，不是数组）
#[derive(Deserialize, Debug)]
pub(crate) struct ChatResponse {
    pub(crate) choices: Vec<Choice>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Choice {
    pub(crate) message: ResponseMessage,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ResponseMessage {
    pub(crate) content: String,
}
