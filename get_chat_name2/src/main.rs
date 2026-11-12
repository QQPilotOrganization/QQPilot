use std::{path::Path, vec};

use image::DynamicImage;
mod completion;

use base64::prelude::*;
use win_screenshot::prelude::*;
mod config;
const CHAT_NAME_BBOX_POSSIBLE_SIZE: (u32, u32, u32, u32) = (237, 46, 534, 40);
const DEFAULT_SIZE: (u32, u32) = (1280, 720);
/// 截下来的图默认存到这里（和 ScreenCapture.dll 用的是同一个名字）。
const DEFAULT_CROP: &str = "cropped.jpg";
/// 默认的配置文件。
const DEFAULT_CONFIG: &str = "config.ini";

/// 自身更多设置的配置文件
const DEFAULT_CONFIG_SELF: &str = "getChatName2.ini";
use base64::engine::general_purpose::STANDARD;
use image::ImageFormat;
use std::io::Cursor;

fn to_data_url(img: &DynamicImage, format: ImageFormat) -> image::ImageResult<String> {
    let mime = match format {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::WebP => "image/webp",
        ImageFormat::Gif => "image/gif",
        _ => "application/octet-stream",
    };
    let b64 = dynamic_image_to_base64(img, format)?;
    Ok(format!("data:{};base64,{}", mime, b64))
}

/// 将 DynamicImage 编码为 base64 字符串（不含 data URL 前缀）
fn dynamic_image_to_base64(img: &DynamicImage, format: ImageFormat) -> image::ImageResult<String> {
    let mut buf = Vec::new();
    // write_to 需要 Write + Seek，所以用 Cursor 包一下 Vec
    img.write_to(&mut Cursor::new(&mut buf), format)?;
    Ok(STANDARD.encode(&buf))
}

const fn to_relative_rect(rect: (u32, u32, u32, u32), size: (u32, u32)) -> (f64, f64, f64, f64) {
    (
        rect.0 as f64 / size.0 as f64,
        rect.1 as f64 / size.1 as f64,
        rect.2 as f64 / size.0 as f64,
        rect.3 as f64 / size.1 as f64,
    )
}
fn chat_name_region(config: &config::Config) -> (i32, i32, i32, i32) {
    let (width, height) = config.pixel_size();
    let (x1, y1, x2, y2) = CHAT_NAME_BBOX_RELATIVE_SIZE;
    let round = |value: f64| value.round() as i32;
    //xywh
    let temp: (i32, i32, i32, i32) = (
        round(x1 * width),
        round(y1 * height),
        round((x2) * width),
        round((y2) * height),
    );
    temp
}
const CHAT_NAME_BBOX_RELATIVE_SIZE: (f64, f64, f64, f64) =
    to_relative_rect(CHAT_NAME_BBOX_POSSIBLE_SIZE, DEFAULT_SIZE);

fn main() {
    let buffer = capture_display().unwrap();
    let image = image::RgbaImage::from_raw(buffer.width, buffer.height, buffer.pixels)
        .expect("failed to convert.");
    let config_section = config::Config::load(Path::new(DEFAULT_CONFIG));
    let config_self = config::ConfigOCR::load(Path::new(if !cfg!(debug_assertions) {
        DEFAULT_CONFIG_SELF
    } else {
        "getChatName2_Dbg.ini"
    }));
    let crop_area = chat_name_region(&config_section);
    if cfg!(debug_assertions) {
        eprintln!(
            "{}~{}~{}",
            config_section.width, config_section.height, config_section.scale
        );
        eprintln!(
            "{}\n{}\n{}",
            config_self.model_name, "sk-**********", config_self.server_url
        );
        eprintln!("crop:{:?}", crop_area);
    }
    let dyn_image = DynamicImage::ImageRgba8(image);
    let cropped_image: DynamicImage = dyn_image.crop_imm(
        crop_area.0 as u32,
        crop_area.1 as u32,
        crop_area.2 as u32,
        crop_area.3 as u32,
    );

    let base64image = to_data_url(&cropped_image, ImageFormat::Jpeg).expect("转换Bases64失败");

    let message = completion::ChatRequest {
        model: config_self.model_name.clone(),
        messages: vec![completion::Message {
            role: String::from("user"),
            content: vec![
                completion::Content::ImageUrl {
                    image_url: completion::ImageUrl {
                        url: base64image.clone(),
                    },
                },
                completion::Content::Text {
                    text: String::from("OCR:"),
                },
            ],
        }],
    };
    match serde_json::to_string(&message) {
        Ok(message_json) => {
            if cfg!(debug_assertions) {
                // println!("{base64image}");
                cropped_image
                    .save_with_format(DEFAULT_CROP, image::ImageFormat::Jpeg)
                    .unwrap(); // println!("{}");

                eprintln!("{message_json}")
            }
            // TLS 用系统自带的（Windows 上就是 SChannel），不用 rustls/ring：
            // ring 要 clang 才能编译，这个项目不想被一个 C 工具链卡住。
            //
            // 两个都必须在 Agent 上显式指定：
            //   * provider 默认还是 Rustls，不改的话 https 会 panic（"feature is not enabled: rustls"）；
            //   * root_certs 默认是 WebPki（只用 Mozilla 内置根），对 native-tls 会
            //     "unable to find any user-specified roots"，改成 PlatformVerifier 用系统根证书。
            let tls = ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::NativeTls)
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .build();
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .http_status_as_error(false) // 4xx/5xx 也当成正常响应，才能读到错误 body
                .tls_config(tls)
                .build()
                .into();
            let mut resp = agent
                .post(&config_self.server_url)
                .header("Authorization", &format!("Bearer {}", config_self.api_key))
                .header("Content-Type", "application/json")
                .send_json(&message)
                .expect("请求发送失败");
            let status = resp.status();
            let body = resp.body_mut().read_to_string().unwrap_or_default();
            if !status.is_success() {
                panic!("HTTP {} {}\n请求地址: {}\n响应: {}", status.as_u16(), status.canonical_reason().unwrap_or(""), config_self.server_url, body);
            }
            match serde_json::from_str::<completion::ChatResponse>(&body) {
                Ok(parsed) => println!("{}", parsed.choices[0].message.content),
                Err(_) => eprintln!("{body}"),
            }
        }
        Err(err) => {
            panic!("{}", err)
        }
    }


}

//不知道该写什么测试
