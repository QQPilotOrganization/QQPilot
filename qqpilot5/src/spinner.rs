//! 旋转加载动画（对应 C# 的 `SpinnerLoad.cs`）。

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::localization;
use crate::log::{self, Color};

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

struct Running {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<()>,
    started: Instant,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

/// 清空当前行（对应 C# 的 `Console.Write("\r" + spaces + "\r")`）。
fn clear_line() {
    let width = log::buffer_width().max(1) - 1;
    print!("\r{}\r", " ".repeat(width as usize));
}

/// 对应 `SpinnerLoad.Start(color, text)`。
pub fn start(color: Color, text: &str) {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let text = text.to_string();

    let handle = thread::spawn(move || {
        let mut index = 0usize;
        while !flag.load(Ordering::Relaxed) {
            let frame = FRAMES[index % FRAMES.len()];
            log::set_color(color);
            let line = format!("{frame} {text}\x1b[0m");
            log::reset();

            clear_line();
            print!("{line}");
            let _ = io::stdout().flush();

            index += 1;
            thread::sleep(Duration::from_millis(100));
        }
    });

    let poisoned = localization::get(&localization::load(), "error.lock.poisoned");
    let mut slot = RUNNING.lock().expect(&poisoned);
    // C# 版没有停掉上一个动画就直接覆盖状态，这里先收尾，避免线程泄漏。
    if let Some(previous) = slot.take() {
        previous.stop.store(true, Ordering::Relaxed);
        let _ = previous.handle.join();
    }
    *slot = Some(Running {
        stop,
        handle,
        started: Instant::now(),
    });
}

/// 对应 `SpinnerLoad.Stop()`。
pub fn stop() {
    let translate = localization::load();
    let poisoned = localization::get(&translate, "error.lock.poisoned");
    let running = RUNNING.lock().expect(&poisoned).take();
    let elapsed = match running {
        Some(running) => {
            running.stop.store(true, Ordering::Relaxed);
            let _ = running.handle.join();
            running.started.elapsed().as_secs_f64()
        }
        None => 0.0,
    };
    println!(
        "\n{}: {elapsed:.2} {}\n",
        localization::get(&translate, "info.spinner.done"),
        localization::get(&translate, "info.second")
    );
}
