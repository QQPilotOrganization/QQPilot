//! QQPilot5

mod answer;
mod arrow_load;
mod chat_content;
mod clipboard;
mod config;
mod conversation;
mod gui_operation;
mod log;
mod native;
mod positions;
mod spinner;
mod tiny_lang_jaccard;
mod upload;
mod upload_content;
mod vision;

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;

use answer::Answer;
use log::Color;
use positions::{PointI, RectI};
use upload_content::UploadContent;
use windows_version::OsVersion;

use crate::sleep::{ ms_output};

/// 自动聚焦线程是否继续运行。
static AUTO_FOCUS_SHOULD_RUN: AtomicBool = AtomicBool::new(true);
/// Ctrl+C 置位，主循环据此收尾退出。
static CANCELLED: AtomicBool = AtomicBool::new(false);

/// 对应 `Console.CancelKeyPress` 的处理：置位后由主循环收尾。
unsafe extern "system" fn console_ctrl_handler(_ctrl_type: u32) -> i32 {
    CANCELLED.store(true, Ordering::SeqCst);
    1
}

mod sleep;

fn main() {
    let settings = config::init();
    // sleep::ms_output(20000);
    // return;
    
    let version_windows=OsVersion::current();
    println!("{}",version_windows.major);
    if version_windows.major>8
    {
        log::enable_utf8_output();
    }

    // 先让 ScaleToINI.exe 把当前 DPI 缩放写回 config.ini，再统一读取配置。
    let mut scale_process = match Command::new("ScaleToINI.exe").spawn() {
        Ok(child) => Some(child),
        Err(e) => {
            log::error(e.to_string());
            None
        }
    };

    arrow_load::start_loading(Color::Green, "正在初始化");

    if let Some(child) = scale_process.as_mut() {
        let _ = child.wait();
    }

    // 对应 GUIOperation.Init()：设置 DPI 感知并加载 InputEvent.dll。
    gui_operation::init();

    let character_name = settings.name.clone();

    let mut token_count: i64 = match fs::read_to_string("tokencount.txt") {
        Ok(text) => match text.trim().trim_start_matches('\u{feff}').parse() {
            Ok(count) => count,
            Err(_) => {
                if let Err(a) = fs::write("tokencount.txt", "0")  {
                    log::error(format!("{a}"),);
                }
                0
            }
        },
        Err(_) => {
            let _ = fs::write("tokencount.txt", "0");
            0
        }
    };

    log::set_color(Color::Cyan);
    log::print(format!("QQPilot {}", settings.version));
    arrow_load::stop_loading();
    log::reset();

    log::print("初始化完成");

    let os_description = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    log::set_color(Color::Yellow);
    log::print(os_description);
    log::reset();

    let auto_focus_thread = thread::spawn(|| {
        while AUTO_FOCUS_SHOULD_RUN.load(Ordering::Relaxed) {
            gui_operation::focus();
            sleep::ms(4000);
        }
    });

    log::set_color(Color::Magenta);
    log::print("请将消息栏拉到最小!");

    log::set_color(Color::Cyan);
    log::print(format!("欢迎您 {character_name}。"));
    log::print("自动聚焦功能已开启");

    if settings.auto_login {
        log::print("自动登录功能已开启");
        log::print("正在尝试登录...");

        for _ in 0..4 {
            vision::full_screenshot();
            let (x, y) = vision::contains_blue();
            if x == 0 && y == 0 {
                sleep::ms(1000);
                continue;
            }
            gui_operation::click(x as i32, y as i32);
            sleep::ms(2000);
        }
        sleep::ms(1000);
    }

    log::set_color(Color::Cyan);
    log::print("======================================================");
    log::print("");
    log::set_color(Color::Yellow);
    log::print("\t使用时请勿移动鼠标！");
    log::print("");
    log::set_color(Color::Cyan);
    log::print("======================================================");

    // 实际窗口尺寸
    let size: (i32, i32) = (
        (settings.width as f32 * settings.scale) as i32,
        (settings.height as f32 * settings.scale) as i32,
    );

    // 聊天列表实际区域
    let chat_list = positions::to_actual_size(positions::CHAT_LIST_BBOX_RELATIVE_SIZE, size);
    // 聊天区域实际区域
    let conversation = positions::to_actual_size(positions::CONVERSATION_BBOX_RELATIVE_SIZE, size);
    // 输入框实际区域
    let comment_section =
        positions::to_actual_size(positions::COMMENT_SECTION_BBOX_RELATIVE_SIZE, size);
    // 拖拽起止位置
    let start_dragging =
        positions::to_actual_point(positions::START_DRAGGING_RELATIVE_POSITION, size);
    let end_dragging = positions::to_actual_point(positions::END_DRAGGING_RELATIVE_POSITION, size);
    // 聊天按钮和联系人按钮位置
    let chat_button = positions::to_actual_point(positions::CHAT_BUTTON_RELATIVE_POSITION, size);
    let contact_button =
        positions::to_actual_point(positions::CONTACT_BUTTON_RELATIVE_POSITION, size);
    // @ 位置实际区域
    let at_place = positions::to_actual_size(positions::AT_PLACE_BBOX_RELATIVE_SIZE, size);
    // 上传图片和复制按钮可能区域
    let upload_image_possible =
        positions::to_actual_size(positions::UPLOAD_IMAGE_POSSIBLE_BBOX_RELATIVE_SIZE, size);
    let copy_button_possible =
        positions::to_actual_size(positions::COPY_BUTTON_POSSIBLE_BBOX_RELATIVE_SIZE, size);

    // SAFETY: 传入的是本文件里 'static 的合法处理函数。
    unsafe { SetConsoleCtrlHandler(Some(console_ctrl_handler), 1) };

    let mut answer_model: Option<Answer> = None;

    while !CANCELLED.load(Ordering::Relaxed) {
        print!("正在寻找新信息...\r");
        let _ = std::io::stdout().flush();

        vision::full_screenshot();
        let mut contain = if settings.at_detect {
            vision::contains_red_dot(vision::rect(at_place))
        } else {
            vision::contains_red_dot(vision::rect(chat_list))
        };

        if contain == (0, 0) {
            sleep::ms(2000);
            continue;
        }

        sleep::ms(500);
        contain = if settings.at_detect {
            vision::contains_red_dot(vision::rect(at_place))
        } else {
            vision::contains_red_dot(vision::rect(chat_list))
        };
        if contain == (0, 0) {
            continue;
        }

        log::set_color(Color::Green);
        log::print(format!("发现红点: {contain:?}"));
        log::reset();

        gui_operation::click(contain.0 as i32, contain.1 as i32);
        sleep::ms(1000);
        gui_operation::drag_from_to_simple(
            start_dragging.0,
            start_dragging.1,
            end_dragging.0,
            end_dragging.1,
        );
        sleep::ms(500);
        gui_operation::goto_center(conversation);
        sleep::ms(500);

        vision::screenshot(copy_button_possible);
        sleep::ms(1000);
        clipboard::set_text("");

        let copy_points = vision::find_templates(vision::SCREENSHOT_FILE, "./copy.png", 30, 1)
            .expect("模板匹配失败");
        if copy_points.is_empty() {
            log::print("使用模板匹配查找复制按钮失败");
            for _ in 0..(settings.scroll * 2) {
                sleep::ms(400);
                gui_operation::scroll_down(480);
            }
            sleep::ms(400);

            gui_operation::click_center(comment_section);
            for _ in 0..settings.tab_times {
                gui_operation::tab();
                sleep::ms(400);
            }
            gui_operation::press_key("enter");
            sleep::ms(200);
        } else {
            sleep::ms(800);
            gui_operation::click(
                (copy_points[0].0 + copy_button_possible.0 as u32) as i32,
                (copy_points[0].1 + copy_button_possible.1 as u32) as i32,
            );
            sleep::ms(1500);
        }

        let chat_content_text = clipboard::get_text();
        if chat_content_text.is_empty() {
            log::error("没有提取到消息。");
            spinner::stop();
            go_back(
                settings.scale,
                chat_button,
                contact_button,
                copy_button_possible,
                upload_image_possible,
                settings.sleep
            );
            
            continue;
        }

        let chat_contents = conversation::parse_chat_log(&chat_content_text, &character_name);
        spinner::start(Color::Green, "等待语言模型生成答案");

        gui_operation::click_center(comment_section);

        let model = answer_model.get_or_insert_with(Answer::new);
        let results = model
            .get_answer(&chat_contents)
            .unwrap_or_else(|| UploadContent::text_only(""));
        let mut result = results.text.clone().unwrap_or_default();
        let images_to_upload = results.absolute();
        if result.chars().count() > 500 {
            result = result.chars().take(500).collect();
        }

        if model.total_tokens != 0 {
            token_count += model.total_tokens;
            log::print(format!("累计用量: {token_count}"));
            if let Err(e) = fs::write("tokencount.txt", token_count.to_string()) {
                log::error(e.to_string());
            }
        }

        spinner::stop();

        let result = result.trim().to_string();
        if result.replace("\n\n", "").is_empty() || result.is_empty() {
            if settings.with_image && settings.send_image_possibility > 0 {
                log::warn("答案未生成,上传图片");
                upload_image_without_send(upload_image_possible);
                gui_operation::hot_key("ctrl", "enter");
                sleep::ms(4000);
                log::print("退出会话");
            } else {
                log::error("答案未生成,退出会话");
            }
            go_back(
                settings.scale,
                chat_button,
                contact_button,
                copy_button_possible,
                upload_image_possible,
                settings.sleep
            );
            continue;
        }

        spinner::stop();
        sleep::ms(100);
        gui_operation::click_center(comment_section);
        gui_operation::send_text(&result, comment_section);

        for image in &images_to_upload {
            log::print("上传获取的图片");
            upload::upload_selected_image(upload_image_possible, image);
            upload::escape();
        }

        let possibility = rand::random_range(0..100);
        log::print(format!("概率:{possibility}"));
        if settings.with_image && possibility < settings.send_image_possibility {
            log::print("上传图片");
            upload_image_without_send(upload_image_possible);
            upload::escape();
        }

        sleep::ms(4000);
        log::print("发送消息 🎉");
        gui_operation::hot_key("ctrl", "enter");
        sleep::ms(4000);
        log::print("退出会话");

        gui_operation::clear_input_section();
        go_back(
            settings.scale,
            chat_button,
            contact_button,
            copy_button_possible,
            upload_image_possible,
            settings.sleep
        );
    }

    log::set_color(Color::Red);
    log::print("\n结束运行");
    log::reset();

    AUTO_FOCUS_SHOULD_RUN.store(false, Ordering::Relaxed);
    let _ = auto_focus_thread.join();
}

/// 对应 `Program.ClearInputSection`。

/// 对应 `Program.UploadImageWithoutSend`：从 exe 目录下的 `Images` 里挑一张图上传。
fn upload_image_without_send(upload_image_possible: RectI) {
    let image_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.join("Images")))
        .unwrap_or_else(|| PathBuf::from("Images"));
    log::print(image_dir.display().to_string());

    let files: Vec<PathBuf> = if image_dir.exists() {
        fs::read_dir(&image_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.path())
                    .collect()
            })
            .unwrap_or_default()
    } else {
        log::error("没有找到图片目录");
        Vec::new()
    };

    let mut contains_image = false;
    for file in &files {
        log::print(file.display().to_string());
        let extension = file
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default();
        if ["jpg", "jpeg", "png", "gif"]
            .iter()
            .any(|known| extension.eq_ignore_ascii_case(known))
        {
            contains_image = true;
            log::print(format!("Image:{}", file.display()));
            break;
        }
    }

    if contains_image {
        upload::upload_image(upload_image_possible);
    }
}

/// 对应 `Program.GoBack`：确保已经退出会话。
fn go_back(
    scale: f32,
    chat_button: PointI,
    _contact_button: PointI,
    copy_button_possible: RectI,
    upload_image_possible: RectI,
    sleeps:u32
) {
    let mut count = 0;
    loop {
        gui_operation::click(
            chat_button.0 + (100.0 * scale) as i32,
            chat_button.1 + (80.0 * scale) as i32,
        );
        sleep::ms(3000);

        if count > 2 {
            break;
        }
        count += 1;

        vision::screenshot(upload_image_possible);
        sleep::ms(1500);
        let upload_points =
            vision::find_templates(vision::SCREENSHOT_FILE, "./uploadImage.png", 30, 1)
                .unwrap_or(vec![]);
        if !upload_points.is_empty() {
            log::print(format!("({},{})", upload_points[0].0, upload_points[0].1));
            continue;
        }

        vision::screenshot(copy_button_possible);
        sleep::ms(1500);
        let copy_points = vision::find_templates(vision::SCREENSHOT_FILE, "./copy.png", 30, 1)
            .unwrap_or(vec![]);
        if !copy_points.is_empty() {
            log::print(format!("({},{})", copy_points[0].0, copy_points[0].1));
            continue;
        }

        break;
    }
    ms_output((sleeps*1000) as u64);
}
