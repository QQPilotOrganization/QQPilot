//! `Windows.Media.Ocr` 的最小封装。
//!
//! 只用 Windows 10 1809+ 自带的 OCR 引擎（需要装好对应语言的 OCR 语言包），
//! 不依赖任何第三方 OCR 库。这一层做三件事：
//!   1. 把图片文件解码成 `SoftwareBitmap`；
//!   2. 需要时按矩形**在解码阶段**裁剪并放大（小字号更容易识别）；
//!   3. 调 `OcrEngine`，把 WinRT 结果整理成纯 Rust 结构体，坐标换算回**原图像素**。
//!
//! 注意：这里的代码是按 WinRT 的公开接口自己写的，没有抄任何第三方封装。

use std::path::Path;

use windows::Globalization::Language;
use windows::Graphics::Imaging::{
    BitmapAlphaMode, BitmapDecoder, BitmapPixelFormat, SoftwareBitmap,
};
use windows::Media::Ocr::OcrEngine;

use crate::bitmap::buffer_from_bytes;
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};
use windows::core::HSTRING;

/// 裁剪区域，单位是原图像素。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// 一块文字的外框，单位是原图像素。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bounds {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// 一个词。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Word {
    pub text: String,
    pub bounds: Bounds,
}

/// 一行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub bounds: Bounds,
    pub words: Vec<Word>,
}

/// 一张图的识别结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// 原图宽（像素）。
    pub width: u32,
    /// 原图高（像素）。
    pub height: u32,
    /// 实际参与识别的区域；整图识别时是整张图。
    pub region: Region,
    /// 为了小字识别放大了几倍（1 表示没放大）。
    pub upscale: u32,
    /// 整段文字（每行之间用换行分隔）。
    pub text: String,
    pub lines: Vec<Line>,
}

/// 小字放大到的最小目标尺寸（像素）。
///
/// Windows 自带的 OCR 对太小的字不敏感，聊天名字那种 30 像素高的条带放大一点更稳。
const TARGET_MIN_SIDE: u32 = 96;
/// 放大倍数上限，别把图撑得太大。
const MAX_UPSCALE: u32 = 4;

/// 一个绑定到某种语言的 OCR 引擎。
pub struct Ocr {
    engine: OcrEngine,
    language: String,
}

impl Ocr {
    /// 系统上装了 OCR 语言包的语言（形如 `zh-Hans-CN`、`en-US`）。
    pub fn available_languages() -> Result<Vec<String>, String> {
        let languages = OcrEngine::AvailableRecognizerLanguages()
            .map_err(|e| format!("读取 OCR 语言列表失败: {e}"))?;
        let count = languages
            .Size()
            .map_err(|e| format!("读取 OCR 语言数量失败: {e}"))?;

        let mut tags = Vec::with_capacity(count as usize);
        for index in 0..count {
            let language = languages
                .GetAt(index)
                .map_err(|e| format!("读取第 {index} 个 OCR 语言失败: {e}"))?;
            let tag = language
                .LanguageTag()
                .map_err(|e| format!("读取语言标签失败: {e}"))?;
            tags.push(tag.to_string());
        }
        Ok(tags)
    }

    /// `language` 为 `None` 时用系统默认（用户配置语言里第一个有 OCR 支持的）。
    pub fn new(language: Option<&str>) -> Result<Self, String> {
        let engine = match language {
            Some(tag) => {
                let language = Language::CreateLanguage(&HSTRING::from(tag))
                    .map_err(|e| format!("语言 {tag} 不是合法的 BCP-47 标签: {e}"))?;
                if !OcrEngine::IsLanguageSupported(&language).unwrap_or(false) {
                    return Err(format!(
                        "系统没有为 {tag} 装 OCR 语言包（用 -s 看看有哪些可用）"
                    ));
                }
                OcrEngine::TryCreateFromLanguage(&language)
                    .map_err(|e| format!("创建 {tag} 的 OCR 引擎失败: {e}"))?
            }
            None => OcrEngine::TryCreateFromUserProfileLanguages().map_err(|e| {
                format!("系统没有可用的 OCR 语言（去「设置 - 语言」里给中文/英文装 OCR）: {e}")
            })?,
        };

        let language = engine
            .RecognizerLanguage()
            .and_then(|language| language.LanguageTag())
            .map(|tag| tag.to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(Self { engine, language })
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    /// 识别一张图片文件。`region` 为 `None` 时识别整张图。
    pub fn recognize(
        &self,
        path: &Path,
        region: Option<Region>,
        upscale: Option<u32>,
    ) -> Result<Document, String> {
        // 读成字节再交给 WinRT 解码。
        //
        // 不用 `StorageFile::GetFileFromPathAsync`：非打包的桌面程序拿它读普通目录里的
        // 文件会时不时报 E_ACCESSDENIED（而且它还不吃 `\\?\` 前缀的路径）。
        let bytes =
            std::fs::read(path).map_err(|e| format!("读图片 {} 失败: {e}", path.display()))?;
        let decoder = decode_bytes(&bytes)?;
        let full = decoder
            .GetSoftwareBitmapConvertedAsync(
                BitmapPixelFormat::Bgra8,
                BitmapAlphaMode::Premultiplied,
            )
            .map_err(|e| format!("转换图片格式失败: {e}"))?
            .get()
            .map_err(|e| format!("转换图片格式失败: {e}"))?;

        self.recognize_bitmap(&full, region, upscale)
    }

    /// 识别一张已经在内存里的位图（屏幕截图就走这条，省掉一次编码/解码）。
    ///
    /// 位图必须是 `Bgra8`（截图和图片解码都会先转成这个格式）。
    /// 返回的坐标一律是**原图像素**（裁剪 + 放大都会换算回去）。
    pub fn recognize_bitmap(
        &self,
        full: &SoftwareBitmap,
        region: Option<Region>,
        upscale: Option<u32>,
    ) -> Result<Document, String> {
        let image_width = full
            .PixelWidth()
            .map_err(|e| format!("读图片宽度失败: {e}"))?;
        let image_height = full
            .PixelHeight()
            .map_err(|e| format!("读图片高度失败: {e}"))?;

        let mut cropped: Option<SoftwareBitmap> = None;
        // 请求的那块区域（不加留白）：OCR 完按它把"顺带看到的邻居文字"过滤掉。
        let mut filter_region: Option<Region> = None;
        let (region, upscale) = match region {
            None => {
                let whole = Region {
                    x: 0,
                    y: 0,
                    width: image_width,
                    height: image_height,
                };
                match upscale.filter(|factor| *factor > 1) {
                    // 整图也允许手动放大（小图上的小字放大后识别率明显更好）。
                    Some(factor) => {
                        let factor = factor.clamp(1, MAX_UPSCALE);
                        cropped = Some(crop_and_scale(full, whole, factor)?);
                        (whole, factor)
                    }
                    None => (whole, 1),
                }
            }
            Some(asked) => {
                let clamped = clamp_region(asked, image_width as u32, image_height as u32)?;
                // 文字贴着上下边缘时 OCR 会一个都读不出来，所以带一圈留白再裁。
                let padded = pad_region(clamped, image_width as u32, image_height as u32);
                let factor = upscale
                    .unwrap_or_else(|| upscale_factor(padded.width as u32, padded.height as u32))
                    .clamp(1, MAX_UPSCALE);
                cropped = Some(crop_and_scale(full, padded, factor)?);
                filter_region = Some(clamped);
                (padded, factor)
            }
        };

        let target = cropped.as_ref().unwrap_or(full);
        let result = self
            .engine
            .RecognizeAsync(target)
            .map_err(|e| format!("OCR 调用失败: {e}"))?
            .get()
            .map_err(|e| format!("OCR 失败: {e}"))?;
        let mut lines = Vec::new();

        let win_lines = result
            .Lines()
            .map_err(|e| format!("读取 OCR 行失败: {e}"))?;
        let line_count = win_lines
            .Size()
            .map_err(|e| format!("读取 OCR 行数失败: {e}"))?;

        for index in 0..line_count {
            let win_line = win_lines
                .GetAt(index)
                .map_err(|e| format!("读取第 {index} 行失败: {e}"))?;
            let raw_text = win_line
                .Text()
                .map_err(|e| format!("读取行文字失败: {e}"))?
                .to_string();
            let text = squeeze_cjk_spaces(&raw_text);

            let win_words = win_line
                .Words()
                .map_err(|e| format!("读取词列表失败: {e}"))?;
            let word_count = win_words.Size().map_err(|e| format!("读取词数失败: {e}"))?;

            let mut words = Vec::with_capacity(word_count as usize);
            let mut union: Option<(f32, f32, f32, f32)> = None;
            for word_index in 0..word_count {
                let win_word = win_words
                    .GetAt(word_index)
                    .map_err(|e| format!("读取第 {word_index} 个词失败: {e}"))?;
                let word_text = win_word
                    .Text()
                    .map_err(|e| format!("读取词文字失败: {e}"))?
                    .to_string();
                let rect = win_word
                    .BoundingRect()
                    .map_err(|e| format!("读取词坐标失败: {e}"))?;

                union = Some(match union {
                    None => (rect.X, rect.Y, rect.X + rect.Width, rect.Y + rect.Height),
                    Some((x1, y1, x2, y2)) => (
                        x1.min(rect.X),
                        y1.min(rect.Y),
                        x2.max(rect.X + rect.Width),
                        y2.max(rect.Y + rect.Height),
                    ),
                });

                words.push(Word {
                    text: word_text,
                    bounds: to_original(rect.X, rect.Y, rect.Width, rect.Height, &region, upscale),
                });
            }

            let bounds = match union {
                Some((x1, y1, x2, y2)) => to_original(x1, y1, x2 - x1, y2 - y1, &region, upscale),
                None => Bounds::default(),
            };

            lines.push(Line {
                text,
                bounds,
                words,
            });
        }

        // 留白会把邻居的文字也带进来；只保留落在请求区域里的行。
        // 万一一行都没落进去（文字稍微越界），那就不过滤，宁可多给也别给空的。
        if let Some(filter) = filter_region {
            let kept: Vec<Line> = lines
                .iter()
                .filter(|line| intersects(&line.bounds, &filter))
                .cloned()
                .collect();
            if !kept.is_empty() {
                lines = kept;
            }
        }

        Ok(Document {
            width: image_width as u32,
            height: image_height as u32,
            region,
            upscale,
            text: lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            lines,
        })
    }
}

/// 把图片字节丢给 WinRT 的解码器。
fn decode_bytes(bytes: &[u8]) -> Result<BitmapDecoder, String> {
    let stream = InMemoryRandomAccessStream::new().map_err(|e| format!("创建内存流失败: {e}"))?;
    let writer =
        DataWriter::CreateDataWriter(&stream).map_err(|e| format!("创建写入器失败: {e}"))?;
    writer
        .WriteBytes(bytes)
        .map_err(|e| format!("写入图片数据失败: {e}"))?;
    writer
        .StoreAsync()
        .and_then(|operation| operation.get())
        .map_err(|e| format!("提交图片数据失败: {e}"))?;
    writer
        .FlushAsync()
        .and_then(|operation| operation.get())
        .map_err(|e| format!("刷新图片数据失败: {e}"))?;
    stream.Seek(0).map_err(|e| format!("回到流开头失败: {e}"))?;

    BitmapDecoder::CreateAsync(&stream)
        .map_err(|e| format!("解码图片失败: {e}"))?
        .get()
        .map_err(|e| format!("解码图片失败（是支持的图片格式吗？）: {e}"))
}

/// 从整图里裁出 `region`，再按 `upscale` 做双线性放大。
///
/// 不用 `BitmapDecoder.GetSoftwareBitmapTransformedAsync`：那个「Bounds + 缩放」的组合
/// 在这台机器上要么返回空白图（OCR 一个字都读不出），要么直接报 E_INVALIDARG，
/// 所以宁可自己按像素搬。
///
/// 放大用双线性而不是最近邻：小字放大后笔画要"变柔和"识别率才高，
/// 最近邻放大出来的锯齿反而更容易把 `l` 认成 `I`。
fn crop_and_scale(
    source: &SoftwareBitmap,
    region: Region,
    upscale: u32,
) -> Result<SoftwareBitmap, String> {
    let width = source
        .PixelWidth()
        .map_err(|e| format!("读图片宽度失败: {e}"))? as usize;
    let height = source
        .PixelHeight()
        .map_err(|e| format!("读图片高度失败: {e}"))? as usize;
    let stride = width * 4;

    let mut pixels = vec![0u8; stride * height];
    let buffer = buffer_from_bytes(&pixels)?;
    source
        .CopyToBuffer(&buffer)
        .map_err(|e| format!("读取图片像素失败: {e}"))?;
    let reader = windows::Storage::Streams::DataReader::FromBuffer(&buffer)
        .map_err(|e| format!("读取像素缓冲区失败: {e}"))?;
    reader
        .ReadBytes(&mut pixels)
        .map_err(|e| format!("拷贝像素失败: {e}"))?;

    let factor = upscale.max(1) as f32;
    let out_width = (region.width as f32 * factor).round() as usize;
    let out_height = (region.height as f32 * factor).round() as usize;
    let mut cropped = vec![0u8; out_width * out_height * 4];

    let sample = |x: f32, y: f32, channel: usize| -> f32 {
        let x = x.clamp(0.0, (width - 1) as f32);
        let y = y.clamp(0.0, (height - 1) as f32);
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(width - 1);
        let y1 = (y0 + 1).min(height - 1);
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;

        let at = |px: usize, py: usize| pixels[py * stride + px * 4 + channel] as f32;
        let top = at(x0, y0) * (1.0 - tx) + at(x1, y0) * tx;
        let bottom = at(x0, y1) * (1.0 - tx) + at(x1, y1) * tx;
        top * (1.0 - ty) + bottom * ty
    };

    for y in 0..out_height {
        let source_y = region.y as f32 + (y as f32 + 0.5) / factor - 0.5;
        let target_row = &mut cropped[y * out_width * 4..(y + 1) * out_width * 4];
        for x in 0..out_width {
            let source_x = region.x as f32 + (x as f32 + 0.5) / factor - 0.5;
            for channel in 0..4 {
                let value = sample(source_x, source_y, channel);
                target_row[x * 4 + channel] = value.round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    let cropped_buffer = buffer_from_bytes(&cropped)?;
    SoftwareBitmap::CreateCopyFromBuffer(
        &cropped_buffer,
        BitmapPixelFormat::Bgra8,
        out_width as i32,
        out_height as i32,
    )
    .map_err(|e| format!("构造裁剪后的图片失败: {e}"))
}

/// 两个矩形有没有重叠（用在同一轴上不重叠就算不相交）。
fn intersects(bounds: &Bounds, region: &Region) -> bool {
    let right = bounds.x + bounds.width;
    let bottom = bounds.y + bounds.height;
    let region_right = region.x + region.width;
    let region_bottom = region.y + region.height;

    bounds.x < region_right && right > region.x && bounds.y < region_bottom && bottom > region.y
}
/// 在裁剪区域外面留一圈空白，再夹进图片范围。
///
/// 实测：同一段文字裁成 750x32 时 Windows OCR 一个字都读不出来，上下各留 16 像素
/// （750x64）就能全读对 —— 文字贴边会让引擎的文字检测失效。所以上下留高度的 1/2、
/// 左右留高度的 1/4。
fn pad_region(region: Region, image_width: u32, image_height: u32) -> Region {
    let vertical = (region.height / 2).max(8);
    let horizontal = (region.height / 4).max(4);

    let left = (region.x - horizontal).max(0);
    let top = (region.y - vertical).max(0);
    let right = (region.x + region.width + horizontal).min(image_width as i32);
    let bottom = (region.y + region.height + vertical).min(image_height as i32);

    Region {
        x: left,
        y: top,
        width: (right - left).max(1),
        height: (bottom - top).max(1),
    }
}

/// 去掉紧挨着文字的空白。
///
/// Windows 的 OCR 把中文按"一个字一个词"返回，`OcrLine.Text` 里就成了
/// `测 试 聊 天 群`；这里把挨着汉字/全角标点的空格去掉，英文单词之间的空格不动。
/// （`words` 里保留原样，需要逐字坐标的调用方可以自己拼。）
fn squeeze_cjk_spaces(text: &str) -> String {
    fn touches_cjk(c: char) -> bool {
        matches!(
            c as u32,
            0x3000..=0x303F   // CJK 标点
            | 0x3400..=0x4DBF // 扩展 A
            | 0x4E00..=0x9FFF // 基本区
            | 0xF900..=0xFAFF // 兼容汉字
            | 0xFF00..=0xFFEF // 全角字符
        )
    }

    let chars: Vec<char> = text.chars().collect();
    let mut squeezed = String::with_capacity(text.len());
    for (index, ch) in chars.iter().copied().enumerate() {
        if ch == ' ' || ch == '\u{3000}' {
            let before = index.checked_sub(1).map(|i| chars[i]);
            let after = chars.get(index + 1).copied();
            let near_cjk =
                before.map(touches_cjk).unwrap_or(false) || after.map(touches_cjk).unwrap_or(false);
            if near_cjk {
                continue;
            }
        }
        squeezed.push(ch);
    }
    squeezed
}
/// 把请求的区域夹进图片范围；完全落在外面就报错。
fn clamp_region(region: Region, image_width: u32, image_height: u32) -> Result<Region, String> {
    if region.width <= 0 || region.height <= 0 {
        return Err(format!(
            "裁剪区域大小不合法: {}x{}",
            region.width, region.height
        ));
    }

    let left = region.x.max(0);
    let top = region.y.max(0);
    let right = (region.x + region.width).min(image_width as i32);
    let bottom = (region.y + region.height).min(image_height as i32);

    if right <= left || bottom <= top {
        return Err(format!(
            "裁剪区域 ({},{}) {}x{} 完全落在图片 {}x{} 之外",
            region.x, region.y, region.width, region.height, image_width, image_height
        ));
    }

    Ok(Region {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

/// 小图放大到 [`TARGET_MIN_SIDE`]，倍数取整并夹在 1..=[`MAX_UPSCALE`]。
fn upscale_factor(width: u32, height: u32) -> u32 {
    let shortest = width.min(height).max(1);
    TARGET_MIN_SIDE.div_ceil(shortest).clamp(1, MAX_UPSCALE)
}

/// OCR 坐标（裁剪后、放大后的像素）→ 原图像素。
fn to_original(x: f32, y: f32, width: f32, height: f32, region: &Region, upscale: u32) -> Bounds {
    let upscale = upscale.max(1) as f32;
    let round = |value: f32| value.round() as i32;
    Bounds {
        x: region.x + round(x / upscale),
        y: region.y + round(y / upscale),
        width: round(width / upscale),
        height: round(height / upscale),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upscale_factors() {
        // 够大就不放大
        assert_eq!(upscale_factor(750, 200), 1);
        // 32 高 -> 3 倍（96）
        assert_eq!(upscale_factor(750, 32), 3);
        // 很小的图也不会超过 4 倍
        assert_eq!(upscale_factor(10, 10), 4);
        // 零尺寸不 panic
        assert_eq!(upscale_factor(0, 0), 4);
    }

    #[test]
    fn intersection_filtering() {
        let region = Region {
            x: 376,
            y: 86,
            width: 750,
            height: 32,
        };

        // 完全在区域里
        assert!(intersects(
            &Bounds {
                x: 380,
                y: 90,
                width: 100,
                height: 24
            },
            &region
        ));
        // 只有部分重叠也算
        assert!(intersects(
            &Bounds {
                x: 370,
                y: 80,
                width: 20,
                height: 10
            },
            &region
        ));
        // 在区域上方
        assert!(!intersects(
            &Bounds {
                x: 380,
                y: 40,
                width: 100,
                height: 24
            },
            &region
        ));
        // 在区域下方
        assert!(!intersects(
            &Bounds {
                x: 380,
                y: 130,
                width: 100,
                height: 24
            },
            &region
        ));
        // 在区域右边
        assert!(!intersects(
            &Bounds {
                x: 1200,
                y: 90,
                width: 100,
                height: 24
            },
            &region
        ));
    }
    #[test]
    fn squeezes_spaces_between_cjk() {
        assert_eq!(squeeze_cjk_spaces("测 试 聊 天 群"), "测试聊天群");
        assert_eq!(squeeze_cjk_spaces("（ 42 ）"), "（42）");
        // 英文单词之间的空格要留着
        assert_eq!(squeeze_cjk_spaces("Hello QQPilot OCR"), "Hello QQPilot OCR");
        // 中英混排：贴着汉字那一侧的空格去掉
        assert_eq!(squeeze_cjk_spaces("QQ 群"), "QQ群");
    }

    #[test]
    fn padding_gives_the_text_some_breathing_room() {
        let padded = pad_region(
            Region {
                x: 376,
                y: 86,
                width: 750,
                height: 32,
            },
            1927,
            1080,
        );
        assert_eq!(
            padded,
            Region {
                x: 368,
                y: 70,
                width: 766,
                height: 64
            }
        );

        // 贴着图片边缘时不会越界
        let padded = pad_region(
            Region {
                x: 0,
                y: 0,
                width: 100,
                height: 20,
            },
            100,
            20,
        );
        assert_eq!(
            padded,
            Region {
                x: 0,
                y: 0,
                width: 100,
                height: 20
            }
        );
    }
    #[test]
    fn clamps_a_region_to_the_image() {
        let clamped = clamp_region(
            Region {
                x: -10,
                y: -10,
                width: 100,
                height: 100,
            },
            500,
            400,
        )
        .unwrap();
        assert_eq!(
            clamped,
            Region {
                x: 0,
                y: 0,
                width: 90,
                height: 90
            }
        );

        let clamped = clamp_region(
            Region {
                x: 450,
                y: 350,
                width: 100,
                height: 100,
            },
            500,
            400,
        )
        .unwrap();
        assert_eq!(
            clamped,
            Region {
                x: 450,
                y: 350,
                width: 50,
                height: 50
            }
        );
    }

    #[test]
    fn a_region_outside_the_image_is_an_error() {
        let error = clamp_region(
            Region {
                x: 600,
                y: 0,
                width: 100,
                height: 100,
            },
            500,
            400,
        )
        .unwrap_err();
        assert!(error.contains("完全落在图片"), "{error}");

        assert!(
            clamp_region(
                Region {
                    x: 0,
                    y: 0,
                    width: 0,
                    height: 10
                },
                500,
                400
            )
            .is_err()
        );
    }

    #[test]
    fn coordinates_are_mapped_back_to_the_original_image() {
        let region = Region {
            x: 376,
            y: 86,
            width: 750,
            height: 32,
        };
        let mapped = to_original(30.0, 15.0, 60.0, 30.0, &region, 3);
        assert_eq!(
            mapped,
            Bounds {
                x: 376 + 10,
                y: 86 + 5,
                width: 20,
                height: 10
            }
        );

        // 不放大时是平移
        let mapped = to_original(30.0, 15.0, 60.0, 30.0, &region, 1);
        assert_eq!(
            mapped,
            Bounds {
                x: 376 + 30,
                y: 86 + 15,
                width: 60,
                height: 30
            }
        );
    }
}
