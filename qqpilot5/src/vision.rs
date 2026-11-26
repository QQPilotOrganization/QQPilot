//! 视觉层封装（对应 C# 的 `Vision.cs`）。
//!
//! 把 `Vision.dll` / `ScreenCapture.dll` 的裸调用包成"截图 -> 检测红点/蓝图 ->
//! 多尺度模板匹配"这一组语义，具体参数来自统一配置模块。

use crate::config;
use crate::localization;
use crate::log;
use crate::native::{self, Point, Rect};

/// 抓屏结果文件名（DLL 内部写死）。
pub const SCREENSHOT_FILE: &str = "screenshot.png";
/// `Vision` 里 `DEFAULT_SCALE` 的取值，用于换算 `dScale`。
const DEFAULT_SCALE: f32 = 1.0;

/// 深色模式下 QQ 的主背景色（对应 `Vision/darkModeCompact.h` 的 `DARK_MODE_MOST_COLOR`）。
const DARK_MODE_BACKGROUND: (u8, u8, u8, u8) = (34, 34, 34, 255);
/// 取主色时的采样比例：缩到 10% 足够判断，速度快很多。
const MOST_COLOR_RESIZE: f32 = 0.1;

/// `dScale = scale / DEFAULT_SCALE`。
fn d_scale() -> f32 {
    config::get().scale / DEFAULT_SCALE
}

/// 对应 `Vision.Rect((int,int,int,int))`。
pub fn rect(area: (i32, i32, i32, i32)) -> Rect {
    native::vision_dll().rect(area.0 as u32, area.1 as u32, area.2 as u32, area.3 as u32)
}

/// 对应 `Vision.Screenshot(x, y, width, height)`。
///
/// DLL 返回 1 表示失败，所以这里取反。
pub fn screenshot_with(x: i32, y: i32, width: i32, height: i32) -> bool {
    native::screen_capture().screenshot(x, y, width, height) != 1
}

/// x1,y1,x2,y2
pub fn screenshot(area: (i32, i32, i32, i32)) -> bool {
    screenshot_with(area.0, area.1, area.2, area.3)
}

/// 对应 `Vision.FullScreenShot()`。
pub fn full_screenshot() -> bool {
    native::screen_capture().full_screenshot() != 1
}

/// 对应 `Vision.ContainsRedDot(rect)`。
pub fn contains_red_dot(rect: Rect) -> (u32, u32) {
    let point = native::vision_dll().contains_red_dot(rect, SCREENSHOT_FILE);
    (point.x, point.y)
}

/// 对应 `Vision.ContainsBlue()`。
pub fn contains_blue() -> (u32, u32) {
    let point = native::vision_dll().contains_blue(SCREENSHOT_FILE);
    (point.x, point.y)
}

fn error_message(code: i32, image_path: &str, template_path: &str) -> String {
    let translate = localization::load();
    let t = |key: &str| localization::get(&translate, key);
    match code {
        -1 => t("error.image.tinier"),
        -2 => format!("{}: {image_path}", t("error.image.loadfailed")),
        -3 => format!("{}: {template_path}", t("error.template.loadfailed")),
        other => format!("{}: {other}", t("error.unknown.code")),
    }
}

/// 对应 `Vision.FindTemplatesByNearingDScale`：在 `dScale` 附近的若干个缩放上依次尝试。
///
/// 7 个倍数一次性交给 DLL：截图只解码一遍，DLL 内部按顺序试、凑够 `max_count` 就停。
/// （以前是每个倍数调一次 `matchTemplatesSelectedScaleBegin`，同一张截图要解码 7 遍。）
pub fn find_templates_by_nearing_d_scale(
    image_path: &str,
    template_path: &str,
    tolerance: i32,
    max_count: i32,
) -> Result<Vec<(u32, u32)>, String> {
    let d = d_scale();
    let scales = [d, d - 0.1, d - 0.2, d + 0.1, d + 0.2, d - 0.05, d + 0.05];

    let vision = native::vision_dll();
    let count = vision.match_scales_begin(image_path, template_path, tolerance, max_count, &scales);
    if count < 0 {
        vision.match_end();
        return Err(error_message(count, image_path, template_path));
    }

    let limit = if max_count > 0 {
        max_count as usize
    } else {
        usize::MAX
    };
    let mut results = Vec::new();
    for index in 0..count {
        let Point { x, y } = vision.match_next(index);
        results.push((x, y));
        if results.len() >= limit {
            break;
        }
    }
    vision.match_end();
    Ok(results)
}

/// 深色模式适配：截图整体是深色背景时先反色一次，浅色模式做的模板才能对上。
///
/// 判断依据就是整张图的主色 —— `getMostColor` 返回 `(34,34,34,255)`（#222222，
/// 也就是 `darkModeCompact.h` 里的 `DARK_MODE_MOST_COLOR`）时才反色，反色结果原地
/// 写回 `image_path`。返回是否真的反色了。
fn adapt_dark_mode(image_path: &str) -> bool {
    let vision = native::vision_dll();
    let most = vision.get_most_color(image_path, MOST_COLOR_RESIZE);
    if (most.r, most.g, most.b, most.a) != DARK_MODE_BACKGROUND {
        return false;
    }

    log::debug(format!(
        "检测到深色模式（主色 {},{},{},{}），反色后再做模板匹配",
        most.r, most.g, most.b, most.a
    ));

    if !vision.invert_image(image_path) {
        log::warn(format!("深色模式反色失败：{image_path}"));
        return false;
    }
    true
}

/// 对应 `Vision.FindTemplates`：先试 `dScale` 邻域，不够再退回多尺度全扫。
pub fn find_templates(
    image_path: &str,
    template_path: &str,
    tolerance: i32,
    max_count: i32,
) -> Result<Vec<(u32, u32)>, String> {
    // 深色模式下先把截图反色回来（模板是浅色模式截的）。
    adapt_dark_mode(image_path);

    let results =
        find_templates_by_nearing_d_scale(image_path, template_path, tolerance, max_count)?;
    // if results.len() >= max_count.max(0) as usize {
    //     return Ok(results);
    // }

    // let vision = native::vision_dll();
    // let count = vision.match_multi_scale_begin(image_path, template_path, tolerance, max_count);
    // if count < 0 {
    //     vision.match_end();
    //     return Err(error_message(count, image_path, template_path));
    // }

    // let limit = if max_count > 0 {
    //     max_count as usize
    // } else {
    //     usize::MAX
    // };
    // let mut results = results;
    // for index in 0..count {
    //     let Point { x, y } = vision.match_next(index);
    //     results.push((x, y));
    //     if results.len() >= limit {
    //         break;
    //     }
    // }
    // vision.match_end();
    Ok(results)
}
/// 在**当前截图**里找模板，成功时返回它在屏幕上的坐标（已加上截图区域的左上角）。
///
/// 截图必须先由调用方用 [`screenshot`] 拍好（本函数只做匹配 + 换算）。
///
/// `require_nonzero`：DLL 找不到时 `matchTemplateNext` 会返回 `(0,0)`，要求非零可以把
/// "没找到"和"刚好命中截图左上角"区分开；有些整块区域的模板（比如录音界面）就该传
/// `false`，因为 `(0,0)` 也可能是真的命中。
pub fn find_template_point(
    template_path: &str,
    area: (i32, i32, i32, i32),
    require_nonzero: bool,
) -> Result<(i32, i32), String> {
    let points = find_templates(SCREENSHOT_FILE, template_path, 30, 1)?;
    let Some(&(x, y)) = points.first() else {
        return Err(format!("截图里没找到 {template_path}"));
    };
    if require_nonzero && (x == 0 || y == 0) {
        return Err(format!("截图里没找到 {template_path}（只拿到 (0,0)）"));
    }
    Ok((x as i32 + area.0, y as i32 + area.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 仓库根目录里就有 `screenshot.png` 和模板，顺便验证新的「一次载图试多个倍数」接口。
    #[test]
    fn match_scales_api_finds_record_icon() {
        let vision = native::vision_dll();
        let count = vision.match_scales_begin("screenshot.png", "record.png", 30, 1, &[1.0]);
        assert!(count >= 1, "应至少命中 1 个，实际 {count}");
        let Point { x, y } = vision.match_next(0);
        vision.match_end();
        assert_eq!((x, y), (118, 5));
    }
}
