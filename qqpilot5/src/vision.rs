//! 视觉层封装（对应 C# 的 `Vision.cs`）。
//!
//! 把 `Vision.dll` / `ScreenCapture.dll` 的裸调用包成"截图 -> 检测红点/蓝图 ->
//! 多尺度模板匹配"这一组语义，具体参数来自统一配置模块。

use crate::config;
use crate::native::{self, Point, Rect};

/// 抓屏结果文件名（DLL 内部写死）。
pub const SCREENSHOT_FILE: &str = "screenshot.png";
/// `Vision` 里 `DEFAULT_SCALE` 的取值，用于换算 `dScale`。
const DEFAULT_SCALE: f32 = 1.0;

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

/// 对应 `Vision.Screenshot((int,int,int,int))`。
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

/// 按 `Vision.dll` 的错误码拼出与 C# 版一致的中文提示。
fn error_message(code: i32, image_path: &str, template_path: &str) -> String {
    match code {
        -1 => "大图尺寸小于模板图尺寸".to_string(),
        -2 => format!("无法加载大图: {image_path}"),
        -3 => format!("无法加载模板图: {template_path}"),
        other => format!("未知错误代码: {other}"),
    }
}

/// 单次指定缩放的匹配，对应 `NativeMethods.matchTemplatesSelectedScaleBegin` 那段逻辑。
fn match_at_scale(
    image_path: &str,
    template_path: &str,
    tolerance: i32,
    max_count: i32,
    scale: f32,
) -> Result<Vec<(u32, u32)>, String> {
    let vision = native::vision_dll();
    let count =
        vision.match_selected_scale_begin(image_path, template_path, tolerance, max_count, scale);
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

/// 对应 `Vision.FindTemplatesByNearingDScale`：在 `dScale` 附近的若干个缩放上依次尝试。
pub fn find_templates_by_nearing_d_scale(
    image_path: &str,
    template_path: &str,
    tolerance: i32,
    max_count: i32,
) -> Result<Vec<(u32, u32)>, String> {
    let d = d_scale();
    let mut results = Vec::new();
    for scale in [d, d - 0.1, d - 0.2, d + 0.1, d + 0.2, d - 0.05, d + 0.05] {
        let found = match_at_scale(image_path, template_path, tolerance, max_count, scale)?;
        results.extend(found);
        if results.len() >= max_count.max(0) as usize {
            break;
        }
    }
    Ok(results)
}

/// 对应 `Vision.FindTemplates`：先试 `dScale` 邻域，不够再退回多尺度全扫。
pub fn find_templates(
    image_path: &str,
    template_path: &str,
    tolerance: i32,
    max_count: i32,
) -> Result<Vec<(u32, u32)>, String> {
    let results =
        find_templates_by_nearing_d_scale(image_path, template_path, tolerance, max_count)?;
    if results.len() >= max_count.max(0) as usize {
        return Ok(results);
    }

    let vision = native::vision_dll();
    let count = vision.match_multi_scale_begin(image_path, template_path, tolerance, max_count);
    if count < 0 {
        vision.match_end();
        return Err(error_message(count, image_path, template_path));
    }

    let limit = if max_count > 0 {
        max_count as usize
    } else {
        usize::MAX
    };
    let mut results = results;
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
