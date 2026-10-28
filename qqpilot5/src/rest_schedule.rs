pub(crate) fn in_schedule(rest_schedule: &str) -> bool {
    // rest_schedule=&rest_schedule.replace("1", "2");
    let mut mut_rest_schadule = String::from(rest_schedule);
    mut_rest_schadule = mut_rest_schadule.replace("，", ",");
    let time = chrono::Local::now();
    let hour = time.format("%H").to_string();
    for selected_hour in mut_rest_schadule.split(",") {
        let st = selected_hour.trim();
        if hour == st {
            return true;
        }
    }
    false
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_string() {
        assert!(!in_schedule("1212312321321312423423"));
        assert!(!in_schedule("False"));
        assert!(!in_schedule("None"));
        assert!(!in_schedule("你好"));
        assert!(!in_schedule("          "));
        assert!(!in_schedule("-4"));
        assert!(!in_schedule("1819"));
    }
    #[test]
    fn selected() {
        let hour = chrono::Local::now().format("%H").to_string();
        assert!(in_schedule(&hour))
    }

    #[test]
    fn multi_select() {
        let mut  hour = chrono::Local::now().format("%H").to_string();
        hour="-1,23123,你好,a,None,False,True,".to_owned()+&hour+",;,--,,sadsa,        ";
        assert!(in_schedule(&hour))
    }
}
