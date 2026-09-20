//! 初始化时的进度条动画（对应 C# 的 `ArrowLoad.cs`）。

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::localization;
use crate::log::{self, Color};

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// 正向进度条帧。
const FORWARD: [&str; 16] = [
    "[->                         ]",
    "[-->                        ]",
    "[ <-->                      ]",
    "[  <-->                     ]",
    "[    <-->                   ]",
    "[      <-->                 ]",
    "[        <-->               ]",
    "[          <-->             ]",
    "[            <-->           ]",
    "[              <-->         ]",
    "[                <-->       ]",
    "[                  <-->     ]",
    "[                    <-->   ]",
    "[                      <--> ]",
    "[                        <--]",
    "[                         <-]",
];

/// 正向 + 反向拼起来的完整动画帧序列。
static PROGRESS_BAR: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut frames: Vec<String> = FORWARD.iter().map(|s| s.to_string()).collect();
    // C# 里去掉首尾两帧后倒序追加，避免出现重复帧。
    for index in (1..FORWARD.len() - 1).rev() {
        frames.push(FORWARD[index].to_string());
    }
    frames
});

struct Running {
    stop: Arc<AtomicBool>,
    handle: JoinHandle<()>,
    started: Instant,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

fn clear_line() {
    let width = log::buffer_width().max(1) - 1;
    print!("\r{}\r", " ".repeat(width as usize));
}

/// 对应 `ArrowLoad.StartLoading(color, text)`。
pub fn start_loading(color: Color, text: &str) {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let text = text.to_string();

    let handle = thread::spawn(move || {
        let mut spinner_index = 0usize;
        let mut bar_index = 0usize;
        while !flag.load(Ordering::Relaxed) {
            let bar = &PROGRESS_BAR[bar_index % PROGRESS_BAR.len()];
            let frame = FRAMES[spinner_index % FRAMES.len()];

            log::set_background_color(color);
            let line = format!("{frame}{bar}\t{text}");
            log::reset();

            clear_line();
            print!("{line}");
            let _ = io::stdout().flush();

            spinner_index += 1;
            bar_index += 1;
            thread::sleep(Duration::from_millis(100));
        }
    });

    let poisoned = localization::get(&localization::load(), "error.lock.poisoned");
    let mut slot = RUNNING.lock().expect(&poisoned);
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

/// 对应 `ArrowLoad.StopLoading()`。
pub fn stop_loading() {
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
        "\n{}: {elapsed:.2}s",
        localization::get(&translate, "info.elapsed")
    );
    println!();
}
