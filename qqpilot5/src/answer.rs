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
use crate::localization;
use crate::log::{self};
use crate::tiny_lang_jaccard::TinyLangJaccard;
use crate::upload_content::UploadContent;
use crate::windows8::windows8;

/// 临时图片目录。
const TEMP_PATH: &str = "./temp";

#[derive(Debug)]
pub struct Answer {
    pub(crate) client: Client,
    pub(crate) model_name: String,
    pub(crate) server_url: String,
    pub(crate) is_vision_model: bool,
    pub(crate) max_image_count: usize,
    pub(crate) api_key: String,
    pub(crate) builtin: bool,
    pub(crate) use_ollama: bool,
    pub(crate) system_prompt: String,
    pub(crate) tiny_lang_jaccard: Option<TinyLangJaccard>,
    /// 对应 `Answer.TotalTokens`，累加历次请求的 token 用量。
    pub total_tokens: i64,
}
/// 一行摘要，方便日志里直接 `{answer}` 看到当前接的是哪个后端、什么参数。
impl std::fmt::Display for Answer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let backend = if self.builtin {
            "builtin"
        } else if self.use_ollama {
            "ollama"
        } else {
            "openai"
        };
        write!(
            f,
            "Answer(backend={backend}, model={}, vision={}, total_tokens={})",
            self.model_name, self.is_vision_model, self.total_tokens
        )
    }
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

        let client_failure = localization::get(&localization::load(), "error.http.client");
        let client = Client::builder()
            .timeout(Duration::from_secs(settings.remote_server_timeout))
            .build()
            .expect(&client_failure);

        let _ = fs::create_dir_all(TEMP_PATH);

        let mut url2: String = server_url;
        while url2.ends_with('/') {
            url2.pop();
        }

        Self {
            client,
            model_name: settings.model_name.clone(),
            server_url: url2,
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

    fn cleanup_temp(&self) {
        let _ = fs::remove_dir_all(TEMP_PATH);
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
        // log::print(serde_json::to_string(extra).unwrap_or_else(|_| "{}".to_string()));

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
        merge_extra(&mut request_body, extra);

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

        log::debug("\n\nResponse:\n");
        log::debug(&response_body);

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

        let settings = config::get();

        // 有没有正文都先把三类附件收下来：图片 / 音频 / 文件。
        // `remote_*` 控制是否允许远程数据，无论是不是远程url或base64都一样。
        let (images, audios, files) = if self.use_ollama {
            // Ollama 的响应结构里没有 images/audios/files 这些字段
            (Vec::new(), Vec::new(), Vec::new())
        } else {
            (
                collect_response(&document, "image", settings.remote_images, &self.client),
                collect_response(&document, "audio", settings.remote_audios, &self.client),
                collect_response(&document, "file", settings.remote_files, &self.client),
            )
        };

        let Some(answer) = answer else {
            log::error(localization::get(
                &localization::load(),
                "error.answer.missing",
            ));

            // 没有正文也照样把附件带回去，主流程可以只发附件
            return Some(UploadContent::new(
                Some(String::from("")),
                images,
                audios,
                files,
            ));
        };

        let elapsed = start_time.elapsed().as_secs_f64();
        let translate = localization::load();
        log::print(format!(
            "{} {elapsed:.2}s",
            localization::get(&translate, "info.elapsed")
        ));
        // log::print(answer.trim());

        Some(UploadContent::new(
            Some(answer.trim().to_string()),
            images,
            audios,
            files,
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
            let uninitialized =
                localization::get(&localization::load(), "error.builtin.uninitialized");
            let answer = self
                .tiny_lang_jaccard
                .as_ref()
                .expect(&uninitialized)
                .answer(&item.text)
                .unwrap_or_else(|e| {
                    log::error(e);
                    std::process::exit(1);
                });
            return Some(UploadContent::text_only(answer));
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
                    log::warn(format!(
                        "× {} {image}",
                        localization::get(&localization::load(), "warning.image.notfound")
                    ));
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
                    log::warn(format!(
                        "× {} {image}",
                        localization::get(&localization::load(), "warning.image.readfailed")
                    ));
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

        if !self.api_key.is_empty() {
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
            let translate = localization::load();
            let t = |key: &str| localization::get(&translate, key);
            log::print(format!(
                "{}: {} {prompt} | {} {completion} | {} {total}",
                t("info.token.usage"),
                t("info.token.input"),
                t("info.token.output"),
                t("info.token.total"),
            ));
            self.total_tokens += total;
        }

        // C# 里这一步被整体 try/catch 包住：Ollama 响应没有 choices 会抛异常，
        // 所以实际上只有 OpenAI 兼容响应的 reasoning_content 会被打印出来。
        let reason = document
            .pointer("/choices/0/message/reasoning_content")
            .and_then(Value::as_str);
        if let Some(reason) = reason {
            log::set_color(log::Color::DarkCyan);
            log::print("============================");
            log::reset();
            if !windows8() {
                log::print(format!("\033[2m<think>\n{reason}\n</think>"));
            } else {
                log::print(format!("<think>\n{reason}\n</think>"));
            }
            log::set_color(log::Color::DarkCyan);
            log::print("============================");
            log::reset();
        }
    }
}

impl Drop for Answer {
    fn drop(&mut self) {
        self.cleanup_temp();
    }
}

/// 把 `extra.json` 的顶层键合并进请求体。
///
/// 放在 [`apply_generation_options`] 之后调用，所以用户可以用 `extra.json`
/// 覆盖 `max_tokens` / `think` 这些字段（与 C# 版的合并顺序一致）。
///
/// 值写成 `null` 表示**把这个字段整个去掉**，而不是发一个 JSON `null`。
/// 有些服务商不认 `think` / `max_tokens` 这类字段，发过去就直接报错，
/// 这时在 `extra.json` 里写 `{"think": null}` 即可。
fn merge_extra(body: &mut Map<String, Value>, extra: &Map<String, Value>) {
    for (key, value) in extra {
        if value.is_null() {
            body.remove(key);
        } else {
            body.insert(key.clone(), value.clone());
        }
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

/// 从 `choices[0].message.<kind>s` 里取出附件，统一落盘成本地文件。
///
/// * `kind`：`"image"` / `"audio"` / `"file"`（数组字段名是它的复数形式）
/// * `allow_remote`：是否允许下载 `http(s)` 链接，对应 config 里的 `remote_*`
/// * `client`：下载用的 HTTP 客户端（复用 `Answer` 的那一个，超时/代理设置一致）
///
/// 每条附件支持两种写法：
///   * `{"image_url": {"url": "..."}}`（音频/文件是 `audio_url` / `file_url`，
///     另外也兼容直接给 `{"url": "..."}`）—— url 可以是 `http(s)` 链接，也可以是
///     `data:<mime>;base64,...`
///   * `{"b64_json": "..."}` —— 裸 base64
fn collect_response(
    document: &Value,
    kind: &str,
    allow_remote: bool,
    client: &Client,
) -> Vec<String> {
    if !allow_remote
    {
        return  vec![];
    }
    let Some(items) = document
        .pointer(&format!("/choices/0/message/{kind}s"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    let translate = localization::load();
    let t = |key: &str| localization::get(&translate, key);

    println!(
        "{} {} {}",
        t(&format!("info.{kind}.found")),
        items.len(),
        t(&format!("info.{kind}.unit"))
    );

    let mut paths = Vec::new();
    for item in items {
        let url = item
            .pointer(&format!("/{kind}_url/url"))
            .and_then(Value::as_str)
            .or_else(|| item.get("url").and_then(Value::as_str));

        if let Some(url) = url {
            let preview: String = url.chars().take(50).collect();
            println!("{}: {preview}...", t(&format!("info.{kind}.dataurl")));

            if is_remote(url) && !allow_remote {
                log::warn(format!("{}: {preview}...", t("warning.remote.disabled")));
            } else if let Some(path) = save_to_temp(url, kind, client) {
                paths.push(path);
            }
        }

        if let Some(encoded) = item.get("b64_json").and_then(Value::as_str) {
            println!(
                "{}: {}",
                t(&format!("info.{kind}.baselength")),
                encoded.len()
            );
            if let Some(path) = save_to_temp(encoded, kind, client) {
                paths.push(path);
            }
        }
    }
    paths
}

/// 是不是一个远程链接（`http://` / `https://`）。
fn is_remote(source: &str) -> bool {
    source.starts_with("http://") || source.starts_with("https://")
}

/// 对应 `Answer.SaveToTemp`：把远程链接 / data URL / base64 存成
/// `./temp/<md5>.<ext>`。
///
/// 远程链接会真的下载一次（超时沿用 `Answer` 的客户端），
/// 文件名取内容的 MD5，扩展名优先看 Content-Type，其次看 URL 路径。
fn save_to_temp(source: &str, kind: &str, client: &Client) -> Option<String> {
    // 测试里可能没走过 `Answer::new`，这里补一次确保目录存在
    let _ = fs::create_dir_all(TEMP_PATH);

    let translate = localization::load();
    let t = |key: &str| localization::get(&translate, key);

    // 1. http(s) 链接：下载
    if is_remote(source) {
        let response = match client.get(source).send() {
            Ok(response) => response,
            Err(e) => {
                log::error(format!("{} {source}: {e}", t("error.download.failed")));
                return None;
            }
        };
        if !response.status().is_success() {
            log::error(format!(
                "{} {source}: HTTP {}",
                t("error.download.failed"),
                response.status()
            ));
            return None;
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| {
                value
                    .split(';')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            });
        let bytes = match response.bytes() {
            Ok(bytes) => bytes,
            Err(e) => {
                log::error(format!("{} {source}: {e}", t("error.download.failed")));
                return None;
            }
        };

        let extension = content_type
            .as_deref()
            .and_then(extension_of_mime)
            .map(str::to_string)
            .or_else(|| extension_of_url(source))
            .unwrap_or_else(|| default_extension(kind).to_string());

        let file_name = format!("{}.{}", md5_hex(source), extension);
        let file_path = Path::new(TEMP_PATH).join(file_name);
        let file_path = file_path.to_string_lossy().into_owned();
        return save_bytes(&bytes, &file_path).map(|()| file_path);
    }

    // 2. data URL：`data:<mime>;base64,<payload>`
    let (meta, payload) = match source.split_once(',') {
        Some((meta, payload)) => (meta, payload),
        None => ("", source),
    };
    let extension = if meta.starts_with("data:") {
        let mime = meta
            .trim_start_matches("data:")
            .split(';')
            .next()
            .unwrap_or_default();
        match extension_of_mime(mime) {
            Some(extension) => extension.to_string(),
            None => {
                log::error(format!("{}: {meta}", t("error.image.unknownmime")));
                return None;
            }
        }
    } else if !meta.is_empty() {
        // 形状不对（既不是链接也不是 data URL，逗号前还有别的内容）
        let preview: String = source.chars().take(60).collect();
        log::error(format!("{}: {preview}", t("error.image.badformat")));
        return None;
    } else {
        // 3. 裸 base64：没有 MIME 信息，按类型给个默认扩展名
        default_extension(kind).to_string()
    };

    let file_name = format!("{}.{}", md5_hex(payload), extension);
    let file_path = Path::new(TEMP_PATH).join(file_name);
    let file_path = file_path.to_string_lossy().into_owned();

    save_base64_image(payload, &file_path)?;
    Some(file_path)
}

/// `Content-Type` / data URL 里的 MIME 对应的文件扩展名。
fn extension_of_mime(mime: &str) -> Option<&'static str> {
    let mime = mime.trim().to_ascii_lowercase();
    let mime = mime.split(';').next().unwrap_or_default().trim();
    Some(match mime {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        "image/avif" => "avif",
        "image/svg+xml" => "svg",
        "audio/mpeg" | "audio/mp3" => "mp3",
        "audio/wav" | "audio/x-wav" | "audio/wave" => "wav",
        "audio/ogg" | "audio/opus" => "ogg",
        "audio/mp4" | "audio/aac" | "audio/x-m4a" => "m4a",
        "audio/flac" | "audio/x-flac" => "flac",
        "audio/webm" => "webm",
        "video/mp4" => "mp4",
        "application/pdf" => "pdf",
        "application/zip" => "zip",
        "application/json" => "json",
        "text/plain" => "txt",
        "application/octet-stream" => "bin",
        _ => return None,
    })
}

/// 从 URL 路径里猜扩展名（`.../a/b.mp3?x=1` -> `mp3`）。
fn extension_of_url(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next().unwrap_or_default();
    let file = path.rsplit('/').next().unwrap_or_default();
    let (_, extension) = file.rsplit_once('.')?;
    if extension.is_empty() || extension.len() > 8 {
        return None;
    }
    if !extension.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(extension.to_ascii_lowercase())
}

/// 拿不到 MIME 时按类型给的默认扩展名。
fn default_extension(kind: &str) -> &'static str {
    match kind {
        "image" => "png",
        "audio" => "mp3",
        "file" => "bin",
        _ => "bin",
    }
}

/// 把字节写进文件；失败只记日志。
fn save_bytes(bytes: &[u8], file_path: &str) -> Option<()> {
    let translate = localization::load();
    match write_file(file_path, bytes) {
        Ok(()) => {
            println!(
                "{}: {file_path}",
                localization::get(&translate, "info.saved")
            );
            Some(())
        }
        Err(e) => {
            log::error(format!(
                "{} {file_path}: {e}",
                localization::get(&translate, "error.file.writefailed")
            ));
            None
        }
    }
}
/// 写文件；`./temp` 可能在写入的瞬间被 `Answer` 的析构删掉，所以最多重试 3 次。
fn write_file(file_path: &str, bytes: &[u8]) -> std::io::Result<()> {
    let mut last = Ok(());
    for _ in 0..3 {
        if let Some(parent) = Path::new(file_path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        match fs::write(file_path, bytes) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => last = Err(e),
            Err(e) => return Err(e),
        }
    }
    last
}

/// 对应 `Answer.SaveBase64Image`。
fn save_base64_image(base64_string: &str, file_path: &str) -> Option<()> {
    let translate = localization::load();
    let t = |key: &str| localization::get(&translate, key);

    log::print(format!("saved{file_path}"));
    let payload = match base64_string.split_once(',') {
        Some((_, rest)) => rest,
        None => base64_string,
    };

    match base64::engine::general_purpose::STANDARD.decode(payload) {
        Ok(bytes) => match write_file(file_path, &bytes) {
            Ok(()) => {
                println!("{}: {file_path}", t("info.image.saved"));
                Some(())
            }
            Err(e) => {
                log::error(format!("{} {file_path}: {e}", t("error.file.writefailed")));
                None
            }
        },
        Err(e) => {
            log::error(format!("{}: {e}", t("error.base64.decode")));
            None
        }
    }
}

/// 对应 `Answer.MD5`，返回小写十六进制。
fn md5_hex(input: &str) -> String {
    let digest = Md5::new().chain_update(input.as_bytes()).finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    /// `./temp` 是进程级共享目录，而 `Answer` 析构会把它整个删掉；
    /// 会写/删它的测试必须串起来，否则偶尔会撞上"目录刚被删"。
    static TEMP_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// `Display` 要能安全地打出后端/参数摘要（这里只是别 panic）。
    #[test]
    fn display_summarizes_the_backend() {
        let _guard = TEMP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let settings = Answer::new();
        let text = settings.to_string();
        assert!(text.starts_with("Answer(backend="), "{text}");
    }

    /// 真机联调用的探针：会真的发一次 HTTP 请求，所以默认忽略。
    ///
    /// ```powershell
    /// $env:sfkey = "<你的 key>"
    /// cargo test answer::tests::live_view -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "会真的发一次 HTTP 请求，需要 sfkey 环境变量"]
    fn live_view() {
        let mut answer = Answer::new();
        answer.server_url = String::from("https://api.siliconflow.cn/v1");
        answer.api_key = env::var("sfkey").expect("需要设置 sfkey 环境变量");
        answer.model_name = String::from("Qwen/Qwen3-8B");
        answer.builtin = false;
        println!("{answer}");

        let chat = ChatContent::new("", vec![], "text", "time", false);
        let _ = answer.get_answer(&[chat]);
    }

    /// 看真实服务商到底收到了什么请求体。
    ///
    /// `Answer` 会在发请求**之前**把请求体落盘成 `dest.json`，所以就算没有
    /// API key（请求必然失败）也能看到实际发出去的字段：
    ///
    /// ```powershell
    /// cargo test answer::tests::view2 -- --ignored --nocapture
    /// ```
    ///
    /// 注意它会真的往服务商发一次请求（也会等 `remote_server_timeout`），
    /// 所以默认忽略，别放进 `cargo test` 的常规路径里。
    #[test]
    #[ignore = "会真的发一次 HTTP 请求，只用于手动看 dest.json"]
    fn view2() {
        let mut answer = Answer::new();
        answer.server_url = String::from("https://example.com/");
        answer.model_name = String::from("Qwen/Qwen3-8B");
        answer.builtin = false;
        println!("{answer}");

        let chat = ChatContent::new("", vec![], "text", "time", false);
        // 没有 api key，请求必然失败；但 dest.json 已经写好了
        let _ = answer.get_answer(&[chat]);

        let dest = std::fs::read_to_string("dest.json").expect("没有写出 dest.json");
        println!("{dest}");

        let body: Value = serde_json::from_str(&dest).expect("dest.json 不是合法 JSON");
        assert_eq!(body.get("model"), Some(&json!("Qwen/Qwen3-8B")));
    }
    /// 远程链接要真的下载下来并落到 `./temp`。
    #[test]
    fn downloads_remote_attachment() {
        let _guard = TEMP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0u8; 1024];
                let _ = stream.read(&mut buffer);
                let body = b"hello-audio";
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: audio/mpeg\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(body);
            }
        });

        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let url = format!("http://127.0.0.1:{port}/whatever");
        let path = save_to_temp(&url, "audio", &client).expect("远程下载失败");
        server.join().unwrap();

        // 扩展名来自 Content-Type，内容原样落盘
        assert!(path.ends_with(".mp3"), "{path}");
        assert_eq!(fs::read(&path).unwrap(), b"hello-audio");
        let _ = fs::remove_file(path);
    }

    /// data URL 现在要能识别任意 MIME（以前只认 image/）。
    #[test]
    fn saves_data_url_with_any_mime() {
        let _guard = TEMP_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        let payload = base64::engine::general_purpose::STANDARD.encode(b"abc");
        let source = format!("data:audio/mpeg;base64,{payload}");
        let path = save_to_temp(&source, "audio", &client).expect("data URL 保存失败");
        assert!(path.ends_with(".mp3"), "{path}");
        assert_eq!(fs::read(&path).unwrap(), b"abc");
        let _ = fs::remove_file(path);

        // 裸 base64：没有 MIME，按类型给默认扩展名
        let path = save_to_temp(&payload, "file", &client).expect("裸 base64 保存失败");
        assert!(path.ends_with(".bin"), "{path}");
        let _ = fs::remove_file(path);
    }
    /// 扩展名猜测：Content-Type 优先，其次 URL 路径，最后按类型兜底。
    #[test]
    fn guesses_extensions() {
        assert_eq!(extension_of_mime("image/png"), Some("png"));
        assert_eq!(extension_of_mime("audio/mpeg"), Some("mp3"));
        assert_eq!(extension_of_mime("application/pdf"), Some("pdf"));
        assert_eq!(extension_of_mime("audio/x-unknown"), None);

        assert_eq!(
            extension_of_url("https://a/b/c.mp3?x=1"),
            Some("mp3".to_string())
        );
        assert_eq!(extension_of_url("https://a/b/c"), None);
        assert_eq!(
            extension_of_url("https://a/b/c.WAV"),
            Some("wav".to_string())
        );

        assert_eq!(default_extension("image"), "png");
        assert_eq!(default_extension("audio"), "mp3");
        assert_eq!(default_extension("file"), "bin");

        assert!(is_remote("https://example.com/a.mp3"));
        assert!(is_remote("http://example.com/a.mp3"));
        assert!(!is_remote("data:image/png;base64,AA"));
        assert!(!is_remote("temp/a.png"));
    }
    #[test]
    fn darken_text() {
        println!("\033[2m<think>\n123123124214\n</think>");
    }

    #[test]
    fn remove_slash() {
        let mut url2: String = String::from("1/////////////////");
        while url2.ends_with('/') {
            url2.pop();
        }
        assert_eq!(url2, String::from("1"), "?")
    }
}
