//! 语言模型调用（对应 C# 的 `Answer.cs`）。
//!
//! 支持三种后端：
//!   * `builtin` —— 内置的 Jaccard 问答（`datasetTiny.json`）
//!   * `ollama` / `forceollamaapi=true` —— Ollama `/api/chat` 协议
//!   * 其它 —— OpenAI 兼容的 `{base}/chat/completions`

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use base64::Engine;
use md5::{Digest, Md5};
use reqwest::blocking::{Client, Response};
use serde_json::{Map, Value, json};

use crate::chat_content::ChatContent;
use crate::config;
use crate::log::{self, Color};
use crate::tiny_lang_jaccard::TinyLangJaccard;
use crate::upload_content::UploadContent;

/// 临时图片目录。
const TEMP_PATH: &str = "./temp";

pub struct Answer {
    client: Client,
    model_name: String,
    server_url: String,
    is_vision_model: bool,
    max_image_count: usize,
    api_key: String,
    builtin: bool,
    use_ollama: bool,
    system_prompt: String,
    tiny_lang_jaccard: Option<TinyLangJaccard>,
    /// 对应 `Answer.TotalTokens`，累加历次请求的 token 用量。
    pub total_tokens: i64,
}

impl Answer {
    /// 对应 `Answer()` 构造函数。
    pub fn new() -> Self {
        let settings = config::get();

        let mut server_url = settings.server_url.clone();
        let mut use_ollama = false;
        let mut builtin = false;

        if settings.force_ollama_api {
            use_ollama = true;
            server_url.push_str("/api/chat");
        } else if server_url.eq_ignore_ascii_case("ollama") {
            server_url = "http://localhost:11434/api/chat".to_string();
            use_ollama = true;
        } else if server_url.eq_ignore_ascii_case("builtin") {
            builtin = true;
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(settings.remote_server_timeout))
            .build()
            .expect("创建 HTTP 客户端失败");

        let _ = fs::create_dir_all(TEMP_PATH);

        Self {
            client,
            model_name: settings.model_name.clone(),
            server_url,
            is_vision_model: settings.is_vision_model,
            max_image_count: settings.max_image_count,
            api_key: settings.api_key.clone(),
            builtin,
            use_ollama,
            system_prompt: settings.system_prompt.clone(),
            tiny_lang_jaccard: None,
            total_tokens: 0,
        }
    }

    /// 对应 C# 的终结器：正常退出时清掉临时目录（非空时删不掉，与 C# 行为一致）。
    fn cleanup_temp(&self) {
        let _ = fs::remove_dir(TEMP_PATH);
    }

    /// 对应 `Answer.GetAnswer(text)`，系统提示词取配置里的 `system.txt`。
    pub fn get_answer(&mut self, text: &[ChatContent]) -> Option<UploadContent> {
        self.get_answer_with(text, "auto")
    }

    /// 对应 `Answer.GetAnswerAsync(text, systemPrompt)`。
    pub fn get_answer_with(
        &mut self,
        text: &[ChatContent],
        system_prompt: &str,
    ) -> Option<UploadContent> {
        let start_time = Instant::now();

        if text.is_empty() {
            return Some(UploadContent::text_only(""));
        }

        if self.builtin {
            return self.answer_builtin(text);
        }

        // extra.json 里的顶层键会整体合并进请求体
        let extra = &config::get().extra;
        log::print(serde_json::to_string(extra).unwrap_or_else(|_| "{}".to_string()));

        let final_system_prompt = match system_prompt {
            "auto" => self.system_prompt.clone(),
            "" | "None" => String::new(),
            other => other.to_string(),
        };

        let images = self.collect_images(text);
        let mut messages: Vec<Value> = Vec::new();
        if !final_system_prompt.is_empty() {
            messages.push(json!({ "role": "system", "content": final_system_prompt }));
        }
        messages.extend(self.concatenate_text(text, &images));

        let mut request_body = Map::new();
        request_body.insert("model".into(), json!(self.model_name));
        request_body.insert("messages".into(), Value::Array(messages));
        request_body.insert("stream".into(), json!(false));
        for (key, value) in extra {
            request_body.insert(key.clone(), value.clone());
        }

        let body = serde_json::to_string(&request_body).unwrap_or_default();
        if let Ok(pretty) = serde_json::to_string_pretty(&request_body) {
            let _ = fs::write("dest.json", pretty);
        }

        let response = match self.post(&body) {
            Ok(response) => response,
            Err(e) => {
                log::error(format!("HTTP request failed: {e}"));
                return None;
            }
        };

        let status = response.status();
        let response_body = response.text().unwrap_or_default();
        if !status.is_success() {
            log::error(format!("API Error: {status} - {response_body}"));
            return None;
        }

        log::print("\n\nResponse:\n");
        log::print(&response_body);

        let document: Value = match serde_json::from_str(&response_body) {
            Ok(value) => value,
            Err(e) => {
                log::error(format!("HTTP request failed: {e}"));
                return None;
            }
        };

        self.log_usage_and_reasoning(&document);

        let answer = if self.use_ollama {
            document.pointer("/message/content").and_then(Value::as_str)
        } else {
            document
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str)
        };
        let Some(answer) = answer else {
            log::error("响应中缺少答案字段");
            return None;
        };

        let outgoing_images = if self.use_ollama {
            Vec::new()
        } else {
            collect_response_images(&document)
        };

        let elapsed = start_time.elapsed().as_secs_f64();
        log::print(format!("用时 {elapsed:.2}s"));
        log::print(answer.trim());

        Some(UploadContent::new(
            Some(answer.trim().to_string()),
            outgoing_images,
        ))
    }

    /// 内置模型分支（对应 `GetAnswerAsync` 里的 `Builtin` 处理）。
    fn answer_builtin(&mut self, text: &[ChatContent]) -> Option<UploadContent> {
        for item in text.iter().rev() {
            if item.text.is_empty() || item.own_by_myself {
                continue;
            }
            if self.tiny_lang_jaccard.is_none() {
                self.tiny_lang_jaccard = Some(
                    TinyLangJaccard::new("datasetTiny.json").unwrap_or_else(|e| {
                        log::error(e);
                        std::process::exit(1);
                    }),
                );
            }
            let answer = self
                .tiny_lang_jaccard
                .as_ref()
                .expect("内置模型已初始化")
                .answer(&item.text)
                .unwrap_or_else(|e| {
                    log::error(e);
                    std::process::exit(1);
                });
            return Some(UploadContent::new(Some(answer), Vec::new()));
        }
        Some(UploadContent::text_only(""))
    }

    /// 对应 `GetAnswerAsync` 里的图片收集循环。
    fn collect_images(&self, text: &[ChatContent]) -> Vec<String> {
        let mut image_list: Vec<String> = Vec::new();
        if !self.is_vision_model {
            return image_list;
        }

        for item in text.iter().rev() {
            if item.own_by_myself {
                continue;
            }
            for image in &item.image_paths {
                if Path::new(image).exists() {
                    image_list.push(image.clone());
                    if image_list.len() >= self.max_image_count {
                        break;
                    }
                } else {
                    log::warn(format!("× 没有找到图片 {image}"));
                }
            }
            if image_list.len() >= self.max_image_count {
                break;
            }
        }
        image_list
    }

    /// 对应 `Answer.ConcatenateText`：把聊天记录转成 API 的 `messages` 数组。
    fn concatenate_text(&self, text_list: &[ChatContent], images: &[String]) -> Vec<Value> {
        let mut messages = Vec::new();

        for item in text_list {
            let mut image_base64: Vec<String> = Vec::new();
            let mut image_data_urls: Vec<String> = Vec::new();

            for image in &item.image_paths {
                if !images.iter().any(|kept| kept == image) {
                    continue;
                }
                let Ok(bytes) = fs::read(image) else {
                    log::warn(format!("× 读取图片失败 {image}"));
                    continue;
                };
                let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
                let mime = mime_of(image);
                image_base64.push(encoded.clone());
                image_data_urls.push(format!("data:{mime};base64,{encoded}"));
            }

            let has_text = !item.text.is_empty();
            let attach_images = !item.own_by_myself && !image_base64.is_empty();
            if !has_text && !attach_images {
                continue;
            }

            let content_text = if has_text {
                item.to_string()
            } else {
                String::new()
            };
            let role = if item.own_by_myself {
                "assistant"
            } else {
                "user"
            };

            let mut message = Map::new();
            message.insert("role".into(), json!(role));

            if self.use_ollama {
                // Ollama：images 与 content 平级，放纯 base64（不带 data: 前缀）
                message.insert("content".into(), json!(content_text));
                if attach_images {
                    message.insert("images".into(), json!(image_base64));
                }
            } else if !attach_images {
                message.insert("content".into(), json!(content_text));
            } else {
                // OpenAI 兼容：content 用分段数组
                let mut parts = vec![json!({ "type": "text", "text": content_text })];
                for url in &image_data_urls {
                    parts.push(json!({ "type": "image_url", "image_url": { "url": url } }));
                }
                message.insert("content".into(), Value::Array(parts));
            }

            messages.push(Value::Object(message));
        }

        messages
    }

    /// 对应 `GetAnswerAsync` 里构造请求、鉴权并 POST 的部分。
    fn post(&self, body: &str) -> Result<Response, reqwest::Error> {
        let url = if self.use_ollama {
            self.server_url.clone()
        } else {
            format!("{}/chat/completions", self.server_url)
        };
        log::print(format!("Sending request to: {url}"));

        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json; charset=utf-8")
            .body(body.to_string());

        if !self.api_key.is_empty()
            && !self.server_url.contains("localhost")
            && !self.server_url.contains("127.0.0.1")
        {
            request = request.header("Authorization", format!("Bearer {}", self.api_key));
        }

        request.send()
    }

    /// 对应响应里 `usage` / `reasoning_content` 的解析（C# 版这整段被 try/catch 包住）。
    fn log_usage_and_reasoning(&mut self, document: &Value) {
        if let Some(usage) = document.get("usage") {
            let read = |key: &str| usage.get(key).and_then(Value::as_i64).unwrap_or_default();
            let (prompt, completion, total) = (
                read("prompt_tokens"),
                read("completion_tokens"),
                read("total_tokens"),
            );
            log::print(format!(
                "Token 用量: 输入 {prompt} | 输出 {completion} | 总计 {total}"
            ));
            self.total_tokens += total;
        }

        // C# 里这一步被整体 try/catch 包住：Ollama 响应没有 choices 会抛异常，
        // 所以实际上只有 OpenAI 兼容响应的 reasoning_content 会被打印出来。
        let reason = document
            .pointer("/choices/0/message/reasoning_content")
            .and_then(Value::as_str);
        if let Some(reason) = reason {
            log::set_color(Color::Gray);
            log::print(format!("<think>\n{reason}\n</think>"));
        }
    }
}

impl Drop for Answer {
    fn drop(&mut self) {
        self.cleanup_temp();
    }
}

/// 按扩展名推断 MIME 类型（与 C# 版的分支完全一致，兜底是 `image/jpeg`）。
fn mime_of(path: &str) -> &'static str {
    let lower = path.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".bmp") {
        "image/bmp"
    } else if lower.ends_with(".avif") {
        "image/avif"
    } else {
        "image/jpeg"
    }
}

/// 对应 `GetAnswerAsync` 里遍历 `choices[0].message.images` 的那段。
fn collect_response_images(document: &Value) -> Vec<String> {
    let Some(images) = document
        .pointer("/choices/0/message/images")
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    println!("找到 {} 张图片", images.len());

    let mut paths = Vec::new();
    for image in images {
        if let Some(url) = image.pointer("/image_url/url").and_then(Value::as_str) {
            let preview: String = url.chars().take(50).collect();
            println!("图片 DataURL: {preview}...");
            if let Some(path) = save_to_temp(url) {
                paths.push(path);
            }
        }
        if let Some(encoded) = image.get("b64_json").and_then(Value::as_str) {
            println!("Base64 长度: {}", encoded.len());
            if let Some(path) = save_to_temp(encoded) {
                paths.push(path);
            }
        }
    }
    paths
}

/// 对应 `Answer.SaveToTemp`：把 data URL / base64 存成 `./temp/<md5>.<ext>`。
fn save_to_temp(base64_string: &str) -> Option<String> {
    let meta = base64_string.split(';').next().unwrap_or_default();
    let Some((_, payload)) = base64_string.split_once(',') else {
        let preview: String = base64_string.chars().take(60).collect();
        log::error(format!("图片数据格式不正确: {preview}"));
        return None;
    };
    let Some(extension) = meta.split("data:image/").nth(1) else {
        log::error(format!("无法识别图片 MIME 前缀: {meta}"));
        return None;
    };

    let file_name = format!("{}.{}", md5_hex(payload), extension);
    let file_path = Path::new(TEMP_PATH).join(file_name);
    let file_path = file_path.to_string_lossy().into_owned();

    save_base64_image(payload, &file_path)?;
    Some(file_path)
}

/// 对应 `Answer.SaveBase64Image`。
fn save_base64_image(base64_string: &str, file_path: &str) -> Option<()> {
    log::print(format!("saved{file_path}"));
    let payload = match base64_string.split_once(',') {
        Some((_, rest)) => rest,
        None => base64_string,
    };

    match base64::engine::general_purpose::STANDARD.decode(payload) {
        Ok(bytes) => match fs::write(file_path, bytes) {
            Ok(()) => {
                println!("图片已保存到: {file_path}");
                Some(())
            }
            Err(e) => {
                log::error(format!("写入 {file_path} 失败: {e}"));
                None
            }
        },
        Err(e) => {
            log::error(format!("base64 解码失败: {e}"));
            None
        }
    }
}

/// 对应 `Answer.MD5`，返回小写十六进制。
fn md5_hex(input: &str) -> String {
    let digest = Md5::new().chain_update(input.as_bytes()).finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
