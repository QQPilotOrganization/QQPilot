use std::io::Write;
use std::{thread, time::Duration};

use crate::localization;

// use crate::log::print;

pub(crate) fn ms(milliseconds: u64) {
    thread::sleep(Duration::from_millis(milliseconds));
}

pub(crate) fn ms_output(milliseconds: u64) {
    let translate = localization::load();
    let seconds: u64 = milliseconds / 1000;
    let l2 = milliseconds % 1000;
    const PROGRESS_WIDTH: u16 = 30;
    println!(
        "{}{}{}",
        localization::get(&translate, "info.waiting"),
        seconds,
        localization::get(&translate, "info.second")
    );
    for i in 0..seconds {
        print!("\r{}\r", " ".repeat(50));
        let elapsed = ((i as f64 / seconds as f64) * PROGRESS_WIDTH as f64).round() as u16;
        let remain = PROGRESS_WIDTH - elapsed;
        print!(
            "{}|{}{}|",
            seconds - i - 1,
            "0".repeat(elapsed.into()),
            " ".repeat(remain.into())
        );
        let _ = std::io::stdout().flush();

        thread::sleep(Duration::from_millis(1000));
    }
    thread::sleep(Duration::from_millis(l2));
}
