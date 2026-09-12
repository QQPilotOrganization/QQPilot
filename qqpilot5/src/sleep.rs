use std::{thread, time::Duration};

pub(crate) fn sleep_ms(milliseconds: u64) {
    thread::sleep(Duration::from_millis(milliseconds));
}
