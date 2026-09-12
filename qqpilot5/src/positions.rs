//! 坐标与尺寸换算（对应 C# 的 `Positions.cs`）。
//!
//! 所有常量都以 2240x1260（或 1280x720）的基准分辨率记录为绝对坐标，
//! 运行时会按 config.ini 的 width/height/scale 换算成实际屏幕坐标。
//!
//! 这里是一张"对照表"：即使当前主流程没用到某几项，也保持与 C# 版一一对应，
//! 方便以后直接取用或微调，因此允许存在暂未使用的常量。

#![allow(dead_code)]

/// 绝对矩形 `(x1, y1, x2, y2)`。
pub type RectI = (i32, i32, i32, i32);
/// 相对矩形 `(x1, y1, x2, y2)`。
pub type RelRect = (f64, f64, f64, f64);
/// 绝对点 `(x, y)`。
pub type PointI = (i32, i32);
/// 相对点 `(x, y)`。
pub type RelPoint = (f64, f64);

/// 基准分辨率。
pub const DEFAULT_SIZE: (i32, i32) = (2240, 1260);
/// 部分按钮使用的第二基准分辨率。
pub const DEFAULT_SIZE2: (i32, i32) = (1280, 720);

// --- 绝对坐标（x1, y1, x2, y2）---

pub const CHAT_LIST_BBOX_ABSOLUTE_SIZE: RectI = (105, 154, 105 + 305, 154 + 1055);
pub const CONVERSATION_BBOX_ABSOLUTE_SIZE: RectI = (421, 163, 421 + 1796, 163 + 734);
pub const SEND_BUTTON_BBOX_ABSOLUTE_SIZE: RectI = (2023, 1167, 2023 + 182, 1167 + 79);
pub const COMMENT_SECTION_BBOX_ABSOLUTE_SIZE: RectI = (427, 1028, 427 + 2, 1028 + 2);
pub const EXIT_CONVERSATION_BBOX_ABSOLUTE_SIZE: RectI = (367, 228, 367 + 31, 228 + 33);
pub const SEND_IMAGE_BBOX_ABSOLUTE_SIZE: RectI = (663, 917, 663 + 44, 917 + 44);
pub const COPY_BUTTON_BBOX_ABSOLUTE_SIZE: RectI = (1698, 1028, 1698 + 52, 1028 + 45);
pub const AT_PLACE_BBOX_ABSOLUTE_SIZE: RectI = (108, 160, 108 + 165, 180 + 1099);
pub const UPLOAD_IMAGE_POSSIBLE_BBOX_ABSOLUTE_SIZE: RectI = (366, 520, 733, 200);
pub const COPY_BUTTON_POSSIBLE_BBOX_ABSOLUTE_SIZE: RectI = UPLOAD_IMAGE_POSSIBLE_BBOX_ABSOLUTE_SIZE;

// --- 拖拽与取消按钮位置 ---

pub const START_DRAGGING_ABSOLUTE_POSITION: PointI = (1898, 882);
pub const END_DRAGGING_ABSOLUTE_POSITION: PointI = (435, 0);
pub const CANCEL_BUTTON_ABSOLUTE_POSITION: PointI = (1325, 697);

/// DEFAULT_SIZE2 下的按钮位置。
pub const CONTACT_BUTTON_ABSOLUTE_POSITION: PointI = (28, 104);
pub const CHAT_BUTTON_ABSOLUTE_POSITION: PointI = (27, 63);

// --- 相对坐标 ---

pub const CHAT_LIST_BBOX_RELATIVE_SIZE: RelRect =
    to_relative_rect(CHAT_LIST_BBOX_ABSOLUTE_SIZE, DEFAULT_SIZE);
pub const CONVERSATION_BBOX_RELATIVE_SIZE: RelRect =
    to_relative_rect(CONVERSATION_BBOX_ABSOLUTE_SIZE, DEFAULT_SIZE);
pub const SEND_BUTTON_BBOX_RELATIVE_SIZE: RelRect =
    to_relative_rect(SEND_BUTTON_BBOX_ABSOLUTE_SIZE, DEFAULT_SIZE);
pub const COMMENT_SECTION_BBOX_RELATIVE_SIZE: RelRect =
    to_relative_rect(COMMENT_SECTION_BBOX_ABSOLUTE_SIZE, DEFAULT_SIZE);
pub const EXIT_CONVERSATION_BBOX_RELATIVE_SIZE: RelRect =
    to_relative_rect(EXIT_CONVERSATION_BBOX_ABSOLUTE_SIZE, DEFAULT_SIZE);
pub const SEND_IMAGE_BBOX_RELATIVE_SIZE: RelRect =
    to_relative_rect(SEND_IMAGE_BBOX_ABSOLUTE_SIZE, DEFAULT_SIZE);
pub const COPY_BUTTON_BBOX_RELATIVE_SIZE: RelRect =
    to_relative_rect(COPY_BUTTON_BBOX_ABSOLUTE_SIZE, DEFAULT_SIZE);
pub const AT_PLACE_BBOX_RELATIVE_SIZE: RelRect =
    to_relative_rect(AT_PLACE_BBOX_ABSOLUTE_SIZE, DEFAULT_SIZE);
pub const UPLOAD_IMAGE_POSSIBLE_BBOX_RELATIVE_SIZE: RelRect =
    to_relative_rect(UPLOAD_IMAGE_POSSIBLE_BBOX_ABSOLUTE_SIZE, DEFAULT_SIZE2);
pub const COPY_BUTTON_POSSIBLE_BBOX_RELATIVE_SIZE: RelRect =
    to_relative_rect(COPY_BUTTON_POSSIBLE_BBOX_ABSOLUTE_SIZE, DEFAULT_SIZE2);

pub const START_DRAGGING_RELATIVE_POSITION: RelPoint =
    to_relative_point(START_DRAGGING_ABSOLUTE_POSITION, DEFAULT_SIZE);
pub const END_DRAGGING_RELATIVE_POSITION: RelPoint =
    to_relative_point(END_DRAGGING_ABSOLUTE_POSITION, DEFAULT_SIZE);
pub const CANCEL_BUTTON_RELATIVE_POSITION: RelPoint =
    to_relative_point(CANCEL_BUTTON_ABSOLUTE_POSITION, DEFAULT_SIZE);
pub const CONTACT_BUTTON_RELATIVE_POSITION: RelPoint =
    to_relative_point(CONTACT_BUTTON_ABSOLUTE_POSITION, DEFAULT_SIZE2);
pub const CHAT_BUTTON_RELATIVE_POSITION: RelPoint =
    to_relative_point(CHAT_BUTTON_ABSOLUTE_POSITION, DEFAULT_SIZE2);

// --- 辅助函数 ---

/// 绝对矩形 -> 相对矩形。
const fn to_relative_rect(rect: RectI, size: (i32, i32)) -> RelRect {
    (
        rect.0 as f64 / size.0 as f64,
        rect.1 as f64 / size.1 as f64,
        rect.2 as f64 / size.0 as f64,
        rect.3 as f64 / size.1 as f64,
    )
}

/// 绝对点 -> 相对点。
const fn to_relative_point(point: PointI, size: (i32, i32)) -> RelPoint {
    (
        point.0 as f64 / size.0 as f64,
        point.1 as f64 / size.1 as f64,
    )
}

/// 相对矩形 -> 实际矩形。
///
/// C# 用的是 `Math.Round`（银行家舍入），所以这里用 `round_ties_even` 保持一致。
pub fn to_actual_size(relative: RelRect, size: (i32, i32)) -> RectI {
    (
        (relative.0 * size.0 as f64).round_ties_even() as i32,
        (relative.1 * size.1 as f64).round_ties_even() as i32,
        (relative.2 * size.0 as f64).round_ties_even() as i32,
        (relative.3 * size.1 as f64).round_ties_even() as i32,
    )
}

/// 相对点 -> 实际点。
pub fn to_actual_point(relative: RelPoint, size: (i32, i32)) -> PointI {
    (
        (relative.0 * size.0 as f64).round_ties_even() as i32,
        (relative.1 * size.1 as f64).round_ties_even() as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_rect_roundtrips_at_default_size() {
        let actual = to_actual_size(CHAT_LIST_BBOX_RELATIVE_SIZE, DEFAULT_SIZE);
        assert_eq!(actual, CHAT_LIST_BBOX_ABSOLUTE_SIZE);
    }

    #[test]
    fn relative_point_roundtrips_at_default_size() {
        let actual = to_actual_point(START_DRAGGING_RELATIVE_POSITION, DEFAULT_SIZE);
        assert_eq!(actual, START_DRAGGING_ABSOLUTE_POSITION);
    }

    #[test]
    fn scales_with_window_size() {
        assert_eq!(
            to_actual_size((0.25, 0.5, 0.75, 1.0), (1280, 720)),
            (320, 360, 960, 720)
        );
        assert_eq!(to_actual_point((0.5, 0.25), (1280, 720)), (640, 180));
    }

    #[test]
    fn rounds_half_to_even_like_math_round() {
        // C# 的 Math.Round 是银行家舍入；Rust 的 f64::round 会进位，这里必须用 round_ties_even。
        assert_eq!(to_actual_size((0.5, 1.5, 2.5, 0.1), (1, 1)), (0, 2, 2, 0));
    }
}
