//! `lr` —— 用 Windows 自带的 OCR 把 QQ 截图里的文字读出来。
//!
//! 两种用法：
//!   * 不带路径：**先截一张全屏图**，再裁出聊天名字那一块
//!     （`CHAT_NAME_BBOX_RELATIVE_SIZE`）做识别；截图默认存成 `screenshot.png`；
//!   * 带目录（或单张图片）：把图片整张识别。
//!
//! 坐标换算要用 `config.ini` 里的 `width` / `height` / `scale`，读法见 [`config`]。
//!
//! 退出码：0 成功；1 参数错误；2 读配置/文件/截图失败；3 有图片识别失败。

mod bitmap;
mod config;
mod ocr;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::{Value, json};

use config::Config;
use ocr::{Bounds, Document, Ocr, Region};

/// 聊天名字的绝对区域（1280x720 基准下的像素）。
const CHAT_NAME_BBOX_POSSIBLE_SIZE: (u32, u32, u32, u32) = (237, 46, 534, 40);
/// 上面那组绝对坐标对应的基准分辨率。
const DEFAULT_SIZE: (u32, u32) = (1280, 720);

/// 截下来的图默认存到这里（和 ScreenCapture.dll 用的是同一个名字）。
const DEFAULT_SCREENSHOT: &str = "screenshot.png";
/// 默认的配置文件。
const DEFAULT_CONFIG: &str = "config.ini";

/// 认得的图片扩展名（Windows 的解码器其实更多，这里只列常见的）。
const IMAGE_EXTENSIONS: [&str; 9] = [
    "png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "webp", "ico",
];

const fn to_relative_rect(rect: (u32, u32, u32, u32), size: (u32, u32)) -> (f64, f64, f64, f64) {
    (
        rect.0 as f64 / size.0 as f64,
        rect.1 as f64 / size.1 as f64,
        rect.2 as f64 / size.0 as f64,
        rect.3 as f64 / size.1 as f64,
    )
}

const CHAT_NAME_BBOX_RELATIVE_SIZE: (f64, f64, f64, f64) =
    to_relative_rect(CHAT_NAME_BBOX_POSSIBLE_SIZE, DEFAULT_SIZE);

/// 命令行参数。
#[derive(Debug)]
struct Options {
    /// 目录（或单张图片）；为空表示先截图再识别聊天名字。
    path: Option<PathBuf>,
    json: bool,
    output: Option<PathBuf>,
    language: Option<String>,
    config: Option<PathBuf>,
    /// 不截图，直接读这个图片文件（调试/回归用）。
    image: Option<PathBuf>,
    /// 截图存到哪里；`None` 表示不存。
    save_screenshot: Option<PathBuf>,
    /// 覆盖聊天名字的裁剪区域（`x,y,w,h`，原图像素），调参用。
    bbox: Option<Region>,
    /// 覆盖自动放大倍数，调参用。
    upscale: Option<u32>,
    show_languages: bool,
    help: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            path: None,
            json: false,
            output: None,
            language: None,
            config: None,
            image: None,
            save_screenshot: Some(PathBuf::from(DEFAULT_SCREENSHOT)),
            bbox: None,
            upscale: None,
            show_languages: false,
            help: false,
        }
    }
}

impl Options {
    fn config_path(&self) -> &Path {
        self.config.as_deref().unwrap_or(Path::new(DEFAULT_CONFIG))
    }
}

fn print_usage() {
    println!(
        "lr —— 用 Windows 自带的 OCR 读文字\n\
         \n\
         用法:\n\
         \x20 lr [选项]            截屏后识别聊天名称\n\
         \x20 lr [选项] <目录>     识别目录里所有图片（png/jpg/bmp/gif/tif/webp）\n\
         \x20 lr [选项] <图片>     识别单张图片\n\
         \n\
         选项:\n\
         \x20 -h, --help            显示帮助\n\
         \x20 -j, --json            输出详细 JSON（每行、每个词的坐标）\n\
         \x20 -o, --output <文件>   写到文件而不是标准输出\n\
         \x20     --save <文件>     截图存到哪里（默认 ./screenshot.png）\n\
         \x20     --no-save         不保存截图\n\
         \x20 -s, --show-languages  列出系统可用的 OCR 语言后退出\n\
        \x20 -l, --language <语言> 指定 OCR 语言（默认跟随系统，例如 zh-Hans-CN)
        "

    );
    if cfg!(debug_assertions) {
        println!("[DEBUG]");
        println!(
        "\x20     --image <文件>    不截图，直接读这个文件（调试用）\n\
         \x20 -c, --config <文件>   配置文件（默认 ./config.ini）\n\
         \x20     --bbox X,Y,W,H   手动指定裁剪区域（原图像素，调参用）\n\
         \x20     --upscale N      手动指定放大倍数（1~8，调参用）"
        );
    }

    println!(
        "例如:\n\
         \x20 lr                     截屏 + 只打印聊天名称\n\
         \x20 lr>o.txt               截屏 + 保存聊天名称\n\
         \x20 lr -j -o name.json     截屏 + 把聊天名称和每个字的坐标写成 JSON\n\
         \x20 lr --no-save           截屏不保存\n\
         \x20 lr screenshots/        批量识别整个目录\n\
         \x20 lr -l en-US a.png      指定英文识别单张图"
    );
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<Options, String> {
    let mut options = Options::default();
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        let mut value = |name: &str| -> Result<String, String> {
            args.next().ok_or_else(|| format!("{name} 后面缺少参数"))
        };

        match arg.as_str() {
            "-h" | "--help" => {
                options.help = true;
                return Ok(options);
            }
            "-j" | "--json" => options.json = true,
            "-s" | "--show-languages" => options.show_languages = true,
            "-o" | "--output" => options.output = Some(PathBuf::from(value("-o")?)),
            "-l" | "--language" => options.language = Some(value("-l")?),
            "--save" => options.save_screenshot = Some(PathBuf::from(value("--save")?)),
            "--no-save" => options.save_screenshot = None,
            "--image" => options.image = Some(PathBuf::from(value("--image")?)),
            "-c" | "--config" => options.config = Some(PathBuf::from(value("-c")?)),
            "--bbox" => {
                let text = value("--bbox")?;
                options.bbox = Some(parse_region(&text)?);
            }
            "--upscale" => {
                let text = value("--upscale")?;
                let factor = text
                    .trim()
                    .parse::<u32>()
                    .map_err(|_| format!("--upscale 要一个整数，收到的是 {text:?}"))?;
                if factor == 0 || factor > 8 {
                    return Err("--upscale 的取值范围是 1~8".to_string());
                }
                options.upscale = Some(factor);
            }
            other if other.starts_with('-') => {
                return Err(format!("不认识的选项: {other}"));
            }
            other => {
                if options.path.is_some() {
                    return Err(format!("只能给一个路径，多出来的是: {other}"));
                }
                options.path = Some(PathBuf::from(other));
            }
        }
    }

    Ok(options)
}

/// 解析 `x,y,w,h`（原图像素）。
fn parse_region(text: &str) -> Result<Region, String> {
    let parts: Vec<&str> = text.split(',').map(str::trim).collect();
    if parts.len() != 4 {
        return Err(format!("--bbox 要写成 x,y,w,h，收到的是 {text:?}"));
    }
    let mut numbers = [0i32; 4];
    for (index, part) in parts.iter().enumerate() {
        numbers[index] = part
            .parse::<i32>()
            .map_err(|_| format!("--bbox 的第 {} 个数不是整数: {part:?}", index + 1))?;
    }
    Ok(Region {
        x: numbers[0],
        y: numbers[1],
        width: numbers[2],
        height: numbers[3],
    })
}

/// 聊天名字那块区域的实际像素坐标。
fn chat_name_region(config: &Config) -> Region {
    let (width, height) = config.pixel_size();
    let (x1, y1, x2, y2) = CHAT_NAME_BBOX_RELATIVE_SIZE;
    let round = |value: f64| value.round() as i32;
    let temp = Region {
        x: round(x1 * width),
        y: round(y1 * height),
        width: round((x2) * width),
        height: round((y2) * height),
    };
    println!(
        "E{},{},{},{}-{}",
        temp.x, temp.y, temp.width, temp.height, config.scale
    );
    temp
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            let extension = extension.to_ascii_lowercase();
            IMAGE_EXTENSIONS.contains(&extension.as_str())
        })
        .unwrap_or(false)
}

/// 目录里的图片，按文件名排序。
fn collect_images(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|e| format!("读不了目录 {}: {e}", directory.display()))?;

    let mut images = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("读目录项失败: {e}"))?;
        let path = entry.path();
        if path.is_file() && is_image(&path) {
            images.push(path);
        }
    }
    images.sort();

    if images.is_empty() {
        return Err(format!(
            "目录 {} 里没有图片（认得 {}）",
            directory.display(),
            IMAGE_EXTENSIONS.join("/")
        ));
    }
    Ok(images)
}

fn bounds_json(bounds: &Bounds) -> Value {
    json!({
        "x": bounds.x,
        "y": bounds.y,
        "width": bounds.width,
        "height": bounds.height,
    })
}

fn document_json(document: &Document) -> Value {
    let lines: Vec<Value> = document
        .lines
        .iter()
        .map(|line| {
            let words: Vec<Value> = line
                .words
                .iter()
                .map(|word| {
                    json!({
                        "text": word.text,
                        "bounds": bounds_json(&word.bounds),
                    })
                })
                .collect();

            json!({
                "text": line.text,
                "bounds": bounds_json(&line.bounds),
                "words": words,
            })
        })
        .collect();

    json!({
        "width": document.width,
        "height": document.height,
        "region": bounds_json(&Bounds {
            x: document.region.x,
            y: document.region.y,
            width: document.region.width,
            height: document.region.height,
        }),
        "upscale": document.upscale,
        "text": document.text,
        "lines": lines,
    })
}

fn write_output(options: &Options, content: &str) -> Result<(), String> {
    match &options.output {
        Some(path) => {
            std::fs::write(path, content)
                .map_err(|e| format!("写入 {} 失败: {e}", path.display()))?;
            eprintln!("已写入 {}", path.display());
            Ok(())
        }
        None => {
            println!("{content}");
            let _ = std::io::stdout().flush();
            Ok(())
        }
    }
}

fn list_languages(options: &Options) -> Result<ExitCode, String> {
    let languages = Ocr::available_languages()?;
    let default = Ocr::new(None).ok().map(|ocr| ocr.language().to_string());

    let mut text = String::new();
    text.push_str(&format!("系统可用的 OCR 语言（{} 个）:\n", languages.len()));
    for language in &languages {
        let mark = if Some(language) == default.as_ref() {
            "  <- 默认"
        } else {
            ""
        };
        text.push_str(&format!("  {language}{mark}\n"));
    }
    if languages.is_empty() {
        text.push_str("  无\n");
    }
    if let Some(wanted) = &options.language {
        let known = languages.iter().any(|language| language == wanted);
        text.push_str(&format!(
            "\n你指定的 {wanted}：{}\n",
            if known { "可用" } else { "不可用" }
        ));
    }

    write_output(options, text.trim_end())?;
    Ok(ExitCode::SUCCESS)
}

fn run_chat_name(options: &Options, ocr: &Ocr) -> Result<(Value, String), String> {
    let config_path = options.config_path();
    let config = Config::load(config_path);
    let region = options.bbox.unwrap_or_else(|| chat_name_region(&config));

    // 默认先截一张全屏图再识别；`--image` 则直接读现成的文件（调试/回归用）。
    let (document, source) = match options.image.as_deref() {
        Some(path) => (
            ocr.recognize(path, Some(region), options.upscale)?,
            json!({
                "source": "file",
                "path": path.display().to_string(),
            }),
        ),
        None => {
            let screen = bitmap::capture_screen()?;
            let saved_to = match &options.save_screenshot {
                Some(target) => {
                    bitmap::save_png(target, &screen)?;
                    eprintln!("已截图 -> {}", target.display());
                    Some(target.display().to_string())
                }
                None => None,
            };
            (
                ocr.recognize_bitmap(&screen, Some(region), options.upscale)?,
                json!({
                    "source": "capture",
                    "saved_to": saved_to,
                }),
            )
        }
    };

    if document.text.trim().is_empty() {
        eprintln!("提示: 没识别出文字（当前引擎语言 {}）", ocr.language());
    }

    let json = json!({
        "language": ocr.language(),
        "mode": "chat-name",
        "config": {
            "path": config_path.display().to_string(),
            "width": config.width,
            "height": config.height,
            "scale": config.scale,
        },
        "screenshot": source,
        "requested_region": bounds_json(&Bounds {
            x: region.x,
            y: region.y,
            width: region.width,
            height: region.height,
        }),
        "images": [{
            "path": options
                .image
                .as_deref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "screen".to_string()),
            "document": document_json(&document),
        }],
    });

    Ok((json, document.text))
}

fn run_images(ocr: &Ocr, path: &Path, upscale: Option<u32>) -> Result<(Value, String), String> {
    let images = if path.is_dir() {
        collect_images(path)?
    } else if path.is_file() {
        vec![path.to_path_buf()]
    } else {
        return Err(format!("找不到 {}（既不是文件也不是目录）", path.display()));
    };

    let mut entries = Vec::with_capacity(images.len());
    let mut texts = Vec::with_capacity(images.len());
    let mut failures = 0;

    for image in &images {
        match ocr.recognize(image, None, upscale) {
            Ok(document) => {
                entries.push(json!({
                    "path": image.display().to_string(),
                    "document": document_json(&document),
                }));
                texts.push((image.clone(), document.text));
            }
            Err(error) => {
                failures += 1;
                eprintln!("识别失败 {}: {error}", image.display());
            }
        }
    }

    if entries.is_empty() {
        return Err("全部失败".to_string());
    }

    let json = json!({
        "language": ocr.language(),
        "mode": "images",
        "images": entries,
        "failed": failures,
    });

    let mut plain = String::new();
    for (index, (image, text)) in texts.iter().enumerate() {
        if index > 0 {
            plain.push('\n');
        }
        plain.push_str(&format!("== {} ==\n", image.display()));
        plain.push_str(text);
        plain.push('\n');
    }

    Ok((json, plain.trim_end().to_string()))
}

fn main() -> ExitCode {
    // 控制台按 UTF-8 显示，中文才不是一堆问号。
    // SAFETY: 只是设置当前进程控制台的输出代码页；失败就保持默认。
    unsafe {
        let _ = windows::Win32::System::Console::SetConsoleOutputCP(65001);
    }
    // 截屏要在"物理像素"的世界里做，否则高分屏上坐标全错。
    bitmap::make_dpi_aware();

    // WinRT 的异步 .get() 需要这个线程先初始化 COM（MTA 即可）。
    // SAFETY: 重复初始化会返回 RPC_E_CHANGED_MODE，忽略即可。
    unsafe {
        let _ = windows::Win32::System::WinRT::RoInitialize(
            windows::Win32::System::WinRT::RO_INIT_MULTITHREADED,
        );
    }

    let options = match parse_args(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(error) => {
            eprintln!("参数错误: {error}");
            print_usage();
            return ExitCode::from(1);
        }
    };

    if options.help {
        print_usage();
        return ExitCode::SUCCESS;
    }

    let result = (|| -> Result<ExitCode, (u16, String)> {
        let ocr = Ocr::new(options.language.as_deref()).map_err(|error| (2, error))?;

        if options.show_languages {
            return list_languages(&options).map_err(|error| (2, error));
        }

        let (json_value, plain) = match options.path.as_deref() {
            None => run_chat_name(&options, &ocr).map_err(|error| (2, error))?,
            Some(path) => run_images(&ocr, path, options.upscale).map_err(|error| (2, error))?,
        };

        let payload = if options.json {
            serde_json::to_string_pretty(&json_value)
                .map_err(|error| (2, format!("生成 JSON 失败: {error}")))?
        } else {
            plain
        };

        write_output(&options, &payload).map_err(|error| (2, error))?;
        Ok(ExitCode::SUCCESS)
    })();

    match result {
        Ok(code) => code,
        Err((code, message)) => {
            eprintln!("错误: {message}");
            ExitCode::from(code as u8)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    //位置被更新过了。
    fn chat_name_region_scales_with_the_window() {
        // 100% 缩放：1285x720 的窗口
        let config = Config {
            width: 1285,
            height: 720,
            scale: 1.0,
        };
        let region = chat_name_region(&config);
        assert_eq!(region.x, 251);
        assert_eq!(region.y, 57);
        assert_eq!(region.width, 500);
        assert_eq!(region.height, 21);

        // 150% 缩放：换算回 1927x1080，正好是那组绝对坐标
        let config = Config {
            width: 1285,
            height: 720,
            scale: 1.5,
        };
        let region = chat_name_region(&config);
        assert_eq!(region.x, 376);
        assert_eq!(region.y, 86);
        assert_eq!(region.width, 750);
        assert_eq!(region.height, 32);
    }

    #[test]
    fn parses_the_flags() {
        let options = parse_args(
            ["-j", "-o", "out.json", "-l", "en-US", "shots"]
                .into_iter()
                .map(String::from),
        )
        .unwrap();
        assert!(options.json);
        assert_eq!(options.output.as_deref(), Some(Path::new("out.json")));
        assert_eq!(options.language.as_deref(), Some("en-US"));
        assert_eq!(options.path.as_deref(), Some(Path::new("shots")));
        assert_eq!(options.config_path(), Path::new(DEFAULT_CONFIG));

        let options = parse_args(["--image", "shot.png"].into_iter().map(String::from)).unwrap();
        assert_eq!(options.image.as_deref(), Some(Path::new("shot.png")));
        // 默认把截图存成 screenshot.png，--no-save 才不存
        assert_eq!(
            Options::default().save_screenshot.as_deref(),
            Some(Path::new(DEFAULT_SCREENSHOT))
        );
        let options = parse_args(["--no-save"].into_iter().map(String::from)).unwrap();
        assert!(options.save_screenshot.is_none());
        let options = parse_args(["--save", "shot.png"].into_iter().map(String::from)).unwrap();
        assert_eq!(
            options.save_screenshot.as_deref(),
            Some(Path::new("shot.png"))
        );
    }

    #[test]
    fn rejects_unknown_and_extra_arguments() {
        assert!(parse_args(["--nope"].into_iter().map(String::from)).is_err());
        assert!(parse_args(["a", "b"].into_iter().map(String::from)).is_err());
        assert!(parse_args(["-o"].into_iter().map(String::from)).is_err());
    }

    #[test]
    fn image_extensions_are_recognised() {
        assert!(is_image(Path::new("a.PNG")));
        assert!(is_image(Path::new("dir/b.jpeg")));
        assert!(!is_image(Path::new("a.txt")));
        assert!(!is_image(Path::new("no-extension")));
    }

    #[test]
    fn document_json_has_the_expected_shape() {
        let document = Document {
            width: 100,
            height: 50,
            region: Region {
                x: 10,
                y: 5,
                width: 80,
                height: 20,
            },
            upscale: 3,
            text: "你好".to_string(),
            lines: vec![ocr::Line {
                text: "你好".to_string(),
                bounds: Bounds {
                    x: 12,
                    y: 6,
                    width: 30,
                    height: 12,
                },
                words: vec![ocr::Word {
                    text: "你好".to_string(),
                    bounds: Bounds {
                        x: 12,
                        y: 6,
                        width: 30,
                        height: 12,
                    },
                }],
            }],
        };

        let value = document_json(&document);
        assert_eq!(value["width"], 100);
        assert_eq!(value["upscale"], 3);
        assert_eq!(value["region"]["height"], 20);
        assert_eq!(value["lines"][0]["text"], "你好");
        assert_eq!(value["lines"][0]["words"][0]["bounds"]["x"], 12);
    }
}
