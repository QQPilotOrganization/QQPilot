//! 「休息时段」判断
//!
//! `config.ini` 的 `rest_schedule` 是逗号分隔的**小时**列表，例如出厂的
//! `0,1,2,3,4,5,6,7,8` 表示 0 点到 8 点之间休息（只看整点，不看分钟）。
//! 列表里写 `8` 还是 `08` 都认，半角/全角逗号都当分隔符。

/// 当前时间是否落在 `rest_schedule` 列出的小时上。
pub(crate) fn in_schedule(rest_schedule: &str) -> bool {
    // `%H` 是补零的两位（"00".."23"）。
    let hour = chrono::Local::now().format("%H").to_string();
    match_hour(rest_schedule, &hour)
}

/// 判断 `hour`（`"00"`..`"23"`，补零与否都行）在不在列表里。
///
/// 单独拆出来是为了能把"补零 / 不补零"测死：如果直接比字符串，`"8"` 永远不等于
/// `"08"`，出厂的 `0,1,2,...,8` 会一次都不生效 —— 而这个坑只有在凌晨 0~9 点
/// 跑测试才会暴露，所以比较逻辑必须能脱离系统时间单独测。
fn match_hour(rest_schedule: &str, hour: &str) -> bool {
    let Ok(current) = hour.trim().parse::<u32>() else {
        return false;
    };

    rest_schedule
        .split([',', '，']) // 半角逗号、全角逗号都认
        .filter_map(|entry| entry.trim().parse::<u32>().ok())
        .any(|selected_hour| selected_hour == current)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 出厂的默认值：0 点到 8 点休息。
    const DEFAULT_SCHEDULE: &str = "0,1,2,3,4,5,6,7,8";

    /// 补零和不补零必须都认 —— 这就是原来的 bug：`%H` 给的是 `"08"`，
    /// 配置里写的是 `"8"`，字符串比较永远不等，休息时段形同虚设。
    #[test]
    fn padding_does_not_matter() {
        assert!(match_hour(DEFAULT_SCHEDULE, "08"), "配置写 8、现在是 08");
        assert!(match_hour(DEFAULT_SCHEDULE, "8"), "两边都不补零");
        assert!(match_hour("08", "08"), "两边都补零");
        assert!(match_hour("08", "8"), "配置补零、现在不补零");
        assert!(match_hour("0", "00"), "0 点也要能对上");
    }

    /// 列表外的小时不能误报。
    #[test]
    fn other_hours_do_not_match() {
        assert!(!match_hour(DEFAULT_SCHEDULE, "09"));
        assert!(!match_hour(DEFAULT_SCHEDULE, "18"));
        assert!(!match_hour("10,11,12", "13"));
        assert!(!match_hour("23", "00"));
    }

    /// 全角逗号、空格都要能处理，混在一堆垃圾里也要挑得出来。
    #[test]
    fn tolerates_separators_spaces_and_junk() {
        assert!(match_hour("a，08 ，b", "08"));
        assert!(match_hour(
            "  -1, 23123, 你好, a, None, False, True, 08 ,;,--,, sadsa, ",
            "08"
        ));
        assert!(match_hour("0, 1 ,2", "01"));
        assert!(!match_hour("", "08"), "空列表");
        assert!(!match_hour(",,,", "08"), "只有分隔符");
        assert!(!match_hour("8.0,08:30,8个", "08"), "不是纯数字的条目要跳过");
    }

    /// `in_schedule` 用的是当前时间，至少不能和它自己的小时对不上。
    #[test]
    fn in_schedule_matches_now() {
        let padded = chrono::Local::now().format("%H").to_string();
        let unpadded = padded.parse::<u32>().unwrap().to_string();

        assert!(in_schedule(&padded), "补零的 {padded} 应该命中");
        assert!(in_schedule(&unpadded), "不补零的 {unpadded} 应该命中");

        let current: u32 = padded.parse().unwrap();
        let other = (current + 5) % 24;
        assert!(!in_schedule(&other.to_string()), "{other} 不该命中");
    }

    /// 不是小时的东西一律不算命中，也不能 panic。
    #[test]
    fn ignores_junk_and_other_hours() {
        for text in [
            "1212312321321312423423",
            "False",
            "None",
            "你好",
            "          ",
            "-4",
            "1819",
            "",
            ",",
            "8.0",
            "08:30",
        ] {
            assert!(!in_schedule(text), "{text:?} 不该命中");
        }
    }
}
