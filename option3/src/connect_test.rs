//! 「连通性测试」：按界面上的参数真发一次最小请求，确认地址、模型名和 key 都能用。
//!
//! 请求的形状与 qqpilot5 的 `Answer` 完全一致（`ollama` / 强制 Ollama ⇒ `/api/chat`，
//! 其它 ⇒ `{base}/chat/completions`，`extra.json` 的顶层键照旧合并进请求体），
//! 这样"测试通过但运行时失败"的情况会少很多。
//!
//! 这一层只做纯逻辑 + 一次 HTTP，不碰界面，所以可以离线测：
//! 纯函数（地址/请求体/响应解析）直接断言，网络路径用 127.0.0.1 上的假服务器跑。

use std::fs;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde_json::{Map, Value, json};

/// 与 qqpilot5 同一个文件名：顶层键会合并进请求体。
pub const EXTRA_FILE: &str = "extra.json";

/// `server_url = ollama` 时实际请求的地址（对应 qqpilot5 里的硬编码）。
const OLLAMA_LOCAL: &str = "http://localhost:11434/api/chat";

/// 测试请求只发一句最短的话，尽量少花 token。
const PROBE_PROMPT: &str = "hi";

/// 非 2xx 时最多回显这么多字符，免得一个 HTML 错误页把消息框撑爆。
const BODY_EXCERPT: usize = 400;

/// 界面上能填的全部参数。
///
/// 刻意做成纯数据：确认框点「确定」之后要把它整个丢给后台线程，
/// 不能把 `nwg` 的控件（非 `Send`）带过去。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    pub server_url: String,
    pub model: String,
    pub api_key: String,
    pub force_ollama_api: bool,
    pub timeout_secs: u64,
}

/// 请求用哪套协议。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Api {
    Ollama,
    OpenAi,
}

/// 测试成功时的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub status: u16,
    pub elapsed: Duration,
    /// 服务端回的那句话。拿不到就是 `None`，不影响"连得上"这个结论。
    pub reply: Option<String>,
}

/// 测试失败的原因。这里只分类型，文案交给界面层翻。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Failure {
    /// 选了内置模型：不联网，没有可测的东西。
    NoEndpoint,
    /// 连不上 / 证书不对 / DNS 失败之类。
    Transport(String),
    /// 等超时了（单独分出来，好说人话）。
    Timeout,
    /// 服务器回了非 2xx。
    Status { status: u16, body: String },
    /// 2xx 但响应里带 `error`（不少服务商这么干）。
    ApiError(String),
    /// 2xx 但不是 JSON —— 多半是地址写成了网页地址。
    NotJson(String),
}

impl Probe {
    /// 与 `qqpilot5::answer::Answer::new` 同序：强制 Ollama → `ollama` → `builtin`。
    ///
    /// 返回 `None` 表示内置模型。
    pub fn api(&self) -> Option<Api> {
        if self.force_ollama_api || self.server_url.eq_ignore_ascii_case("ollama") {
            Some(Api::Ollama)
        } else if self.server_url.eq_ignore_ascii_case("builtin") {
            None
        } else {
            Some(Api::OpenAi)
        }
    }

    /// 真正要 POST 的地址；内置模型返回 `None`。
    ///
    /// 末尾的 `/` 会被去掉：`{base}//chat/completions` 虽然多数服务器能忍，
    /// 但有的反向代理会 404。
    pub fn endpoint(&self) -> Option<String> {
        let base = self.server_url.trim_end_matches('/');
        match self.api()? {
            // 强制 Ollama 且地址填的就是 "ollama" 时，走的仍是本机默认端口。
            Api::Ollama if self.server_url.eq_ignore_ascii_case("ollama") => {
                Some(OLLAMA_LOCAL.to_string())
            }
            Api::Ollama => Some(format!("{base}/api/chat")),
            Api::OpenAi => Some(format!("{base}/chat/completions")),
        }
    }
}

/// 拼请求体：与 qqpilot5 一样是 `model` / `messages` / `stream` + `extra.json`。
///
/// 只有一条 user 消息，不发系统提示词 —— 测试要的是"能不能打通"，不是答案质量。
pub fn request_body(probe: &Probe, extra: &Map<String, Value>) -> String {
    let mut body = Map::new();
    body.insert("model".into(), json!(probe.model));
    body.insert(
        "messages".into(),
        json!([{ "role": "user", "content": PROBE_PROMPT }]),
    );
    body.insert("stream".into(), json!(false));
    merge_extra(&mut body, extra);
    serde_json::to_string(&body).unwrap_or_default()
}

/// 把 `extra.json` 的顶层键合并进请求体。
///
/// 值写成 `null` 表示**把这个字段整个去掉**（与 qqpilot5 的 `merge_extra` 一致），
/// 有些服务商不认 `think` / `max_tokens` 这类字段，写 `{"think": null}` 就能摘掉。
fn merge_extra(body: &mut Map<String, Value>, extra: &Map<String, Value>) {
    for (key, value) in extra {
        if value.is_null() {
            body.remove(key);
        } else {
            body.insert(key.clone(), value.clone());
        }
    }
}

/// 读 `extra.json` 的顶层对象；缺失 / 空文件 / 格式错误都当空表。
pub fn load_extra() -> Map<String, Value> {
    load_extra_from(EXTRA_FILE)
}

/// 同 [`load_extra`]，但路径由调用方给（测试用，免得依赖工作目录里的那份）。
pub fn load_extra_from(path: impl AsRef<std::path::Path>) -> Map<String, Value> {
    let Ok(text) = fs::read_to_string(path) else {
        return Map::new();
    };
    match serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
        Ok(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

/// 发一次测试请求。内置模型返回 [`Failure::NoEndpoint`]，不会发任何东西。
pub fn run(probe: &Probe) -> Result<Outcome, Failure> {
    run_with(probe, &load_extra())
}

/// 同 [`run`]，但 `extra.json` 由调用方给（测试用）。
pub fn run_with(probe: &Probe, extra: &Map<String, Value>) -> Result<Outcome, Failure> {
    let Some(url) = probe.endpoint() else {
        return Err(Failure::NoEndpoint);
    };
    let Some(api) = probe.api() else {
        return Err(Failure::NoEndpoint);
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(probe.timeout_secs.max(1)))
        .build()
        .map_err(|e| Failure::Transport(error_chain(&e)))?;

    let mut request = client
        .post(&url)
        .header("Content-Type", "application/json; charset=utf-8")
        .body(request_body(probe, extra));

    // 与 qqpilot5 一致：本机地址不发 key（本地 Ollama 不认也不校验）。
    let local = url.contains("localhost") || url.contains("127.0.0.1");
    if !probe.api_key.is_empty() && !local {
        request = request.header("Authorization", format!("Bearer {}", probe.api_key));
    }

    let start = Instant::now();
    let response = request.send().map_err(|e| {
        if e.is_timeout() {
            Failure::Timeout
        } else {
            Failure::Transport(error_chain(&e))
        }
    })?;

    let status = response.status();
    let body = response.text().unwrap_or_default();
    let elapsed = start.elapsed();

    if !status.is_success() {
        return Err(Failure::Status {
            status: status.as_u16(),
            body: excerpt(&body),
        });
    }

    let document: Value =
        serde_json::from_str(&body).map_err(|_| Failure::NotJson(excerpt(&body)))?;
    if let Some(error) = api_error(&document) {
        return Err(Failure::ApiError(error));
    }

    Ok(Outcome {
        status: status.as_u16(),
        elapsed,
        reply: extract_reply(api, &document),
    })
}

/// 从响应里取答复文本；两套协议各认自己的路径。
pub fn extract_reply(api: Api, document: &Value) -> Option<String> {
    let raw = match api {
        Api::Ollama => document.pointer("/message/content").and_then(Value::as_str),
        // `choices[0].text` 是老的 completions 形状，个别网关会这么回。
        Api::OpenAi => document
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .or_else(|| document.pointer("/choices/0/text").and_then(Value::as_str)),
    }?;
    let answer = raw.trim();
    (!answer.is_empty()).then(|| answer.to_string())
}

/// 有些服务商 HTTP 200 但把错误塞在 `error` 里（字符串或 `{message}`）。
fn api_error(document: &Value) -> Option<String> {
    match document.get("error")? {
        Value::String(text) => Some(text.clone()),
        Value::Object(map) => map
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| Some(Value::Object(map.clone()).to_string())),
        other => Some(other.to_string()),
    }
}

/// 截断一段可能要展示给用户的响应体。
fn excerpt(body: &str) -> String {
    let text = body.trim();
    if text.chars().count() <= BODY_EXCERPT {
        return text.to_string();
    }
    let head: String = text.chars().take(BODY_EXCERPT).collect();
    format!("{head}…")
}

/// 把错误链拼成一句话。
///
/// `reqwest` 顶层只说"发请求失败（地址）"，真正的原因（DNS、拒绝连接、证书…）
/// 在 `source()` 里。顶层那句还会重复界面上已经显示的地址，所以直接丢掉。
fn error_chain(error: &dyn std::error::Error) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut current = Some(error);
    while let Some(err) = current {
        let text = err.to_string();
        if !text.is_empty() && !text.contains(" for url (") {
            parts.push(text);
        }
        current = err.source();
    }
    parts.join(": ")
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;

    use super::*;

    fn probe(url: &str) -> Probe {
        Probe {
            server_url: url.to_string(),
            model: "test-model".to_string(),
            api_key: String::new(),
            force_ollama_api: false,
            timeout_secs: 5,
        }
    }

    /// 起一个"只伺候一次"的假服务器，返回端口和"收到的请求报文"的收件口。
    ///
    /// 响应体由 `response` 给（已经是完整的 HTTP 报文）。
    fn serve_once(response: &'static str) -> (u16, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("绑定本机端口失败");
        let port = listener.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();

        std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let request = read_request(&mut stream);
            let _ = tx.send(request);
            let _ = stream.write_all(response.as_bytes());
            let _ = stream.flush();
        });

        (port, rx)
    }

    /// 读到请求头和 `Content-Length` 指定的 body 为止（reqwest 可能分几次写）。
    fn read_request(stream: &mut std::net::TcpStream) -> String {
        let mut buf: Vec<u8> = Vec::new();
        let mut chunk = [0u8; 1024];

        loop {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
            }

            let text = String::from_utf8_lossy(&buf).into_owned();
            let Some(head_end) = text.find("\r\n\r\n") else {
                continue;
            };
            let length: usize = text[..head_end]
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse().ok())?
                })
                .unwrap_or(0);

            if buf.len() >= head_end + 4 + length {
                return text;
            }
        }

        String::from_utf8_lossy(&buf).into_owned()
    }

    /// 拼一个带 `Content-Length`、回完就断开的响应。
    fn http_response(status: &str, body: &str) -> String {
        format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{body}",
            body.len()
        )
    }

    /// `serve_once` 要 `&'static str`（后台线程拿不到借用），测试里直接泄漏一份。
    fn leak(text: String) -> &'static str {
        Box::leak(text.into_boxed_str())
    }

    // ---- 地址 / 协议推断 ----

    #[test]
    fn builtin_needs_no_network() {
        let mut p = probe("builtin");
        assert_eq!(p.api(), None);
        assert_eq!(p.endpoint(), None);
        // 真跑也不该发请求，只报"没有可测的地址"。
        assert_eq!(run_with(&p, &Map::new()), Err(Failure::NoEndpoint));
        p.force_ollama_api = true;
        assert_eq!(p.api(), Some(Api::Ollama));
    }

    #[test]
    fn ollama_alias_points_at_the_local_default_port() {
        let p = probe("OLLAMA");
        assert_eq!(p.api(), Some(Api::Ollama));
        assert_eq!(p.endpoint().as_deref(), Some(OLLAMA_LOCAL));
    }

    #[test]
    fn openai_base_appends_chat_completions_and_trims_slash() {
        for base in ["https://api.example.com/v1", "https://api.example.com/v1/"] {
            let p = probe(base);
            assert_eq!(p.api(), Some(Api::OpenAi));
            assert_eq!(
                p.endpoint().as_deref(),
                Some("https://api.example.com/v1/chat/completions")
            );
        }
    }

    #[test]
    fn forced_ollama_appends_api_chat() {
        let mut p = probe("http://192.168.1.7:11434");
        p.force_ollama_api = true;
        assert_eq!(
            p.endpoint().as_deref(),
            Some("http://192.168.1.7:11434/api/chat")
        );
    }

    // ---- 请求体 ----

    #[test]
    fn body_matches_the_runtime_shape() {
        let body: Value =
            serde_json::from_str(&request_body(&probe("https://x/v1"), &Map::new())).unwrap();

        assert_eq!(body["model"], json!("test-model"));
        assert_eq!(body["stream"], json!(false));
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["messages"][0]["role"], json!("user"));
        assert_eq!(body["messages"][0]["content"], json!(PROBE_PROMPT));
    }

    #[test]
    fn extra_json_can_override_a_field() {
        let mut extra = Map::new();
        extra.insert("temperature".into(), json!(0));
        let body: Value =
            serde_json::from_str(&request_body(&probe("https://x/v1"), &extra)).unwrap();
        assert_eq!(body["temperature"], json!(0));
    }

    /// `null` 是"删掉这个字段"，不是发一个 JSON null —— 这正是给不认
    /// `think` / `max_tokens` 的服务商准备的逃生口。
    #[test]
    fn extra_json_null_removes_a_field() {
        let mut extra = Map::new();
        extra.insert("stream".into(), Value::Null);
        let body: Value =
            serde_json::from_str(&request_body(&probe("https://x/v1"), &extra)).unwrap();
        assert!(body.get("stream").is_none(), "{body}");
    }

    #[test]
    fn missing_or_broken_extra_json_is_an_empty_table() {
        // 用 `target/test-tmp` 里的文件，不依赖工作目录里那份真正的 extra.json
        // （用户随时会往里填参数）。
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp");
        std::fs::create_dir_all(&dir).unwrap();
        let write = |name: &str, content: &str| {
            let path = dir.join(name);
            std::fs::write(&path, content).unwrap();
            path
        };

        // 空文件（仓库里的 extra.json 就是空的）/ 坏 JSON / 顶层不是对象
        assert_eq!(load_extra_from(write("extra-empty.json", "")), Map::new());
        assert_eq!(
            load_extra_from(write("extra-broken.json", "{ not json")),
            Map::new()
        );
        assert_eq!(
            load_extra_from(write("extra-array.json", "[1, 2]")),
            Map::new()
        );
        assert_eq!(load_extra_from(dir.join("extra-absent.json")), Map::new());

        // 正常文件要读出来（顶层键会被合并进请求体）
        let extra = load_extra_from(write("extra-ok.json", r#"{"temperature": 0}"#));
        assert_eq!(extra.get("temperature"), Some(&json!(0)));
    }

    // ---- 响应解析 ----

    #[test]
    fn reply_comes_from_the_right_path() {
        let openai: Value =
            serde_json::from_str(r#"{"choices":[{"message":{"content":" hi "}}]}"#).unwrap();
        assert_eq!(extract_reply(Api::OpenAi, &openai).as_deref(), Some("hi"));

        let ollama: Value = serde_json::from_str(r#"{"message":{"content":"yo"}}"#).unwrap();
        assert_eq!(extract_reply(Api::Ollama, &ollama).as_deref(), Some("yo"));

        // 空答复 / 缺字段都算"没拿到"，不是失败。
        let empty: Value = serde_json::from_str(r#"{"message":{"content":"  "}}"#).unwrap();
        assert_eq!(extract_reply(Api::Ollama, &empty), None);
    }

    #[test]
    fn http_200_with_an_error_field_is_a_failure() {
        let document: Value =
            serde_json::from_str(r#"{"error":{"message":"invalid api key"}}"#).unwrap();
        assert_eq!(api_error(&document).as_deref(), Some("invalid api key"));
    }

    #[test]
    fn excerpt_is_capped() {
        let long = "x".repeat(BODY_EXCERPT + 50);
        assert_eq!(excerpt(&long).chars().count(), BODY_EXCERPT + 1);
        assert_eq!(excerpt("  ok  "), "ok");
    }

    /// 错误链要能翻到真正的原因，同时不重复界面上已经显示的地址。
    #[test]
    fn error_chain_reaches_the_root_cause() {
        #[derive(Debug)]
        struct Inner;
        impl std::fmt::Display for Inner {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "tcp connect error: 拒绝连接 (os error 10061)")
            }
        }
        impl std::error::Error for Inner {}

        #[derive(Debug)]
        struct Outer(Inner);
        impl std::fmt::Display for Outer {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(
                    f,
                    "error sending request for url (http://x/v1/chat/completions)"
                )
            }
        }
        impl std::error::Error for Outer {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }

        assert_eq!(
            error_chain(&Outer(Inner)),
            "tcp connect error: 拒绝连接 (os error 10061)"
        );
    }

    // ---- 真发一次（打本机假服务器） ----

    #[test]
    fn loopback_success_reports_status_and_reply() {
        // 后台线程要 'static，所以响应体先 leak 出来。
        let response = leak(http_response(
            "200 OK",
            r#"{"choices":[{"message":{"content":"pong"}}]}"#,
        ));
        let (port, requests) = serve_once(response);

        let mut p = probe(&format!("http://127.0.0.1:{port}/v1"));
        p.api_key = "secret".to_string();
        let outcome = run_with(&p, &Map::new()).expect("应该能连通");

        assert_eq!(outcome.status, 200);
        assert_eq!(outcome.reply.as_deref(), Some("pong"));

        let request = requests.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(
            request.starts_with("POST /v1/chat/completions"),
            "{request}"
        );
        // 本机地址不带 key（与运行时一致）
        assert!(!request.contains("Authorization"), "{request}");
        assert!(request.contains("\"test-model\""), "{request}");
    }

    #[test]
    fn loopback_status_error_is_reported_with_body() {
        let response = leak(http_response("401 Unauthorized", "bad api key"));
        let (port, _requests) = serve_once(response);
        let p = probe(&format!("http://127.0.0.1:{port}"));

        match run_with(&p, &Map::new()) {
            Err(Failure::Status { status, body }) => {
                assert_eq!(status, 401);
                assert!(body.contains("bad api key"), "{body}");
            }
            other => panic!("期望状态码错误，实际是 {other:?}"),
        }
    }

    #[test]
    fn refused_connection_is_a_transport_error() {
        // 绑一下再放开，端口基本不可能被占回来 —— 连上去必然被拒。
        let port = {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.local_addr().unwrap().port()
        };
        let p = probe(&format!("http://127.0.0.1:{port}"));

        match run_with(&p, &Map::new()) {
            Err(Failure::Transport(_)) => {}
            other => panic!("期望连不上，实际是 {other:?}"),
        }
    }

    #[test]
    fn non_json_success_is_reported() {
        let response = leak(http_response("200 OK", "<html>not an api</html>"));
        let (port, _requests) = serve_once(response);
        let p = probe(&format!("http://127.0.0.1:{port}"));

        match run_with(&p, &Map::new()) {
            Err(Failure::NotJson(body)) => assert!(body.contains("<html>"), "{body}"),
            other => panic!("期望 NotJson，实际是 {other:?}"),
        }
    }
}
