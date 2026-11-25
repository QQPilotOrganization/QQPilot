//! QQPilot5

// localization.rs 的测试用 json! 展开整张翻译表，默认递归上限不够。
#![recursion_limit = "256"]

mod answer;
mod audio;
mod chat_content;
mod clipboard;
mod config;
mod conversation;
mod gui_operation;
mod log;
mod native;
mod positions;
mod rest_schedule;
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

mod job_guard;

use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;

use answer::Answer;
use log::Color;
use positions::{PointI, RectI};
use upload_content::UploadContent;
// use windows_version::OsVersion;

use crate::gui_operation::press_key;
// use crate::cached_point::CachedPoint;
use crate::sleep::ms_output;
use crate::windows8::windows8;

/// 自动聚焦线程是否继续运行。
static AUTO_FOCUS_SHOULD_RUN: AtomicBool = AtomicBool::new(true);
/// Ctrl+C 置位，主循环据此收尾退出。
static CANCELLED: AtomicBool = AtomicBool::new(false);

/// Ctrl+C
unsafe extern "system" fn console_ctrl_handler(_ctrl_type: u32) -> i32 {
    CANCELLED.store(true, Ordering::SeqCst);
    1
}

mod localization;
mod sleep;
mod windows8;

use anyhow::Result;
// mod cached_point;
fn main() {
    let translate = localization::load();

    let settings = config::init();
    // sleep::ms_output(20000);
    // return;

    // let version_windows = OsVersion::current();
    // println!("{}", version_windows.major);
    // if version_windows.major > 8 {
    //     log::enable_utf8_output();
    // }
    if windows8() {
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

    log::print(localization::get(&translate, "default.loading"));
    if let Some(child) = scale_process.as_mut() {
        let _ = child.wait();
    }

    gui_operation::init();

    // 右下角的运行状态浮窗。放进 Job 里，QQPilot 退出（包括 Ctrl+C）时一起结束。
    // `--parent` 只是兜底：万一 Job 没生效，浮窗也会盯着本进程自己退出。
    let overlay = job_guard::JobGuard::create_and_assign(
        &job_guard::resolve_program("InfoRunning2.exe"),
        &[], // &["--parent", &std::process::id().to_string()],
    );
    let _overlay_guard = match overlay {
        Ok(guard) => Some(guard),
        Err(error) => {
            log::warn(format!(
                "{}: {error}",
                localization::get(&translate, "error.overlay.launch")
            ));
            None
        }
    };

    let character_name = settings.name.clone();

    let mut token_count: i64 = match fs::read_to_string("tokencount.txt") {
        Ok(text) => match text.trim().trim_start_matches('\u{feff}').parse() {
            Ok(count) => count,
            Err(_) => {
                if let Err(a) = fs::write("tokencount.txt", "0") {
                    log::error(format!("{a}"));
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
    log::print(format!(
        "{} {}",
        localization::get(&translate, "program.name"),
        settings.version
    ));
    // arrow_load::stop_loading();
    log::reset();

    log::print(localization::get(&translate, "default.loading.success"));

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

    log::set_color(Color::Cyan);
    log::print("======================================================");
    log::print("");
    log::set_color(Color::Yellow);
    log::print(format!(
        "\t{}",
        localization::get(&translate, "warning.nevermovemouse")
    ));
    log::print("");
    log::set_color(Color::Cyan);
    log::print("======================================================");

    log::set_color(Color::Magenta);
    log::print(localization::get(&translate, "warning.minimum.bar"));

    log::set_color(Color::Cyan);
    log::print(format!(
        "{} {character_name}。",
        localization::get(&translate, "info.welcome")
    ));
    log::print(localization::get(&translate, "info.autofocus"));

    if settings.auto_login {
        log::print(localization::get(&translate, "info.autologin"));
        log::print(localization::get(&translate, "info.login.try"));

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

    // 实际窗口尺寸
    let size: (i32, i32) = (
        (settings.width as f32 * settings.scale) as i32,
        (settings.height as f32 * settings.scale) as i32,
    );

    log::debug(format!("window size : {:?}", size));

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

    //录音按钮和进入录音模式的按钮可能区域
    let record_audio_possible =
        positions::to_actual_size(positions::RECORD_AUDIO_POSSIBLE_BBOX_RELATIVE_SIZE, size);

    let _qq_window_size = positions::to_actual_size((0.0, 0.0, 1.0, 1.0), size);
    let recording_audio_possible =
    //qq_window_size;
    positions::to_actual_size(positions::RECORDING_AUDIO_POSSIBLE_BBOX_RELATIVE_SIZE, size);
    // SAFETY: 传入的是本文件里 'static 的合法处理函数。
    unsafe { SetConsoleCtrlHandler(Some(console_ctrl_handler), 1) };

    let mut answer_model: Option<Answer> = None;
    // let mut copy_points_cache = CachedPoint::new_expired();
    while !CANCELLED.load(Ordering::Relaxed) {
        if rest_schedule::in_schedule(&settings.rest_schedule) {
            print!("{}\r", localization::get(&translate, "info.rest.schedule"));
            let _ = std::io::stdout().flush();
            sleep::s(60);
            continue;
        }
        print!(
            "{}\r",
            localization::get(&translate, "info.searchingnewmessage")
        );
        let _ = std::io::stdout().flush();

        let mut contain = contains_red_dot(settings, chat_list, at_place);

        if contain == (0, 0) {
            sleep::ms(2000);
            continue;
        }

        //防抖
        sleep::ms(500);
        contain = contains_red_dot(settings, chat_list, at_place);
        if contain == (0, 0) {
            continue;
        }

        log::set_color(Color::Green);
        log::print(format!(
            "{}: {contain:?}",
            localization::get(&translate, "info.found.reddot")
        ));
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
        // panic!("{:?}",copy_button_possible);
        clipboard::set_text("");

        let copy_points: Vec<(u32, u32)> =
            vision::find_templates(vision::SCREENSHOT_FILE, "./copy.png", 30, 1).unwrap_or_else(
                |_| {
                    panic!(
                        "{}",
                        localization::get(&translate, "error.matchtemplate.failed")
                    )
                },
            );
        if copy_points.is_empty() && settings.tab_after_match_template_failed {
            log::print(localization::get(&translate, "error.matchtemplate.failed"));
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
        } else if !copy_points.is_empty() {
            sleep::ms(800);
            gui_operation::click(
                (copy_points[0].0 + copy_button_possible.0 as u32) as i32,
                (copy_points[0].1 + copy_button_possible.1 as u32) as i32,
            );
            sleep::ms(1500);
        } else {
            log::error(localization::get(
                &translate,
                "warning.message.notextracted",
            ));
            spinner::stop();
            go_back(
                settings,
                chat_button,
                contact_button,
                copy_button_possible,
                upload_image_possible,
                recording_audio_possible,
            );
            continue;
        }

        let chat_content_text = clipboard::get_text();
        if chat_content_text.is_empty() {
            log::error(localization::get(
                &translate,
                "warning.message.notextracted",
            ));
            spinner::stop();
            go_back(
                settings,
                chat_button,
                contact_button,
                copy_button_possible,
                upload_image_possible,
                recording_audio_possible,
            );

            continue;
        }

        let chat_contents = conversation::parse_chat_log(&chat_content_text, &character_name);
        spinner::start(
            Color::Green,
            &localization::get(&translate, "info.waitinganswer"),
        );

        gui_operation::click_center(comment_section);
        gui_operation::clear_input_section();

        let model = answer_model.get_or_insert_with(Answer::new);
        let results = model
            .get_answer(&chat_contents)
            .unwrap_or_else(|| UploadContent::text_only(""));
        let mut result = results.text.clone().unwrap_or_default();
        let images_to_upload = results.absolute_images();
        let audios_to_upload = results.absolute_audios();
        let files_to_upload = results.absolute_files();
        if result.chars().count() > settings.max_output.into() {
            result = result.chars().take(settings.max_output.into()).collect();
        }

        if model.total_tokens != 0 {
            token_count += model.total_tokens;
            log::print(format!(
                "{}: {token_count}",
                localization::get(&translate, "info.tokencount")
            ));
            if let Err(e) = fs::write("tokencount.txt", token_count.to_string()) {
                log::error(e.to_string());
            }
        }

        spinner::stop();

        let result = result.trim().to_string();
        if result.replace("\n\n", "").is_empty() || result.is_empty() {
            if results.has_attachments() {
                log::info(format!(
                    "{},{}",
                    localization::get(&translate, "warning.notgenerated"),
                    localization::get(&translate, "info.uploadgotmedia")
                ));
                upload_attachments(
                    settings.scale,
                    &translate,
                    upload_image_possible,
                    record_audio_possible,
                    recording_audio_possible,
                    &images_to_upload,
                    &audios_to_upload,
                    &files_to_upload,
                );
            } else if settings.with_image && settings.send_image_possibility > 0 {
                log::warn(format!(
                    "{},{}",
                    localization::get(&translate, "warning.notgenerated"),
                    localization::get(&translate, "info.uploadimage")
                ));
                upload_image_without_send(upload_image_possible);
                send::send_message();
                sleep::ms(4000);
                upload::escape();
                log::print(localization::get(&translate, "info.quitconversation"));
            } else {
                log::error(format!(
                    "{}，{}",
                    localization::get(&translate, "warning.notgenerated"),
                    localization::get(&translate, "info.quitconversation")
                ));
            }

            go_back(
                settings,
                chat_button,
                contact_button,
                copy_button_possible,
                upload_image_possible,
                recording_audio_possible,
            );
            continue;
        }

        spinner::stop();
        sleep::ms(100);
        gui_operation::click_center(comment_section);
        gui_operation::send_text(&result, comment_section);

        upload_attachments(
            settings.scale,
            &translate,
            upload_image_possible,
            record_audio_possible,
            recording_audio_possible,
            &images_to_upload,
            &audios_to_upload,
            &files_to_upload,
        );
        sleep::ms(1000);
        let possibility = rand::random_range(0..100);
        // log::print(format!("概率:{possibility}"));
        if settings.with_image && possibility < settings.send_image_possibility {
            log::print(localization::get(&translate, "info.uploadimage"));
            upload_image_without_send(upload_image_possible);
            upload::escape();
        }

        sleep::ms(4000);
        log::print(localization::get(&translate, "info.sentmessage"));
        send::send_message();
        sleep::ms(4000);
        log::print(localization::get(&translate, "info.quitconversation"));

        gui_operation::clear_input_section();
        go_back(
            settings,
            chat_button,
            contact_button,
            copy_button_possible,
            upload_image_possible,
            recording_audio_possible,
        );
    }

    log::set_color(Color::Red);
    log::print("\n");
    log::print(localization::get(&translate, "info.quit"));
    log::reset();

    AUTO_FOCUS_SHOULD_RUN.store(false, Ordering::Relaxed);
    let _ = auto_focus_thread.join();
}

fn contains_red_dot(
    settings: &config::Config,
    chat_list: (i32, i32, i32, i32),
    at_place: (i32, i32, i32, i32),
) -> (u32, u32) {
    let mut contain = if settings.at_detect {
        // vision::screenshot(at_place);
        vision::screenshot(at_place);
        sleep::ms(200);

        vision::contains_red_dot(vision::rect((
            0,
            0,
            at_place.2 - at_place.0,
            at_place.3 - at_place.1,
        )))
    } else {
        vision::screenshot(chat_list);
        sleep::ms(200);
        vision::contains_red_dot(vision::rect((
            0,
            0,
            chat_list.2 - chat_list.0,
            chat_list.3 - chat_list.1,
        )))
    };
    if contain == (0, 0) {
        return (0, 0);
    }
    contain.0 += (if settings.at_detect {
        at_place.0
    } else {
        chat_list.0
    })
    .unsigned_abs();
    contain.1 += (if settings.at_detect {
        at_place.1
    } else {
        chat_list.1
    })
    .unsigned_abs();
    contain
}

/// 把模型回复里带的附件逐个上传并发送
/// 文件使用外部exe:UplaodFiles
/// 依次上传一轮回复带的所有附件：图片 → 文件 → 音频。
#[allow(clippy::too_many_arguments)]
fn upload_attachments(
    scale: f32,
    translate: &serde_json::Value,
    upload_image_possible: RectI,
    record_audio_possible: RectI,
    recording_audio_possible: RectI,
    images: &[String],
    audios: &[String],
    files: &[String],
) {
    log::debug(format!("images:{:?}", images));
    log::debug(format!("audios:{:?}", audios));
    log::debug(format!("files:{:?}", files));
    //图片
    for path in images {
        log::print(localization::get(translate, "info.uploadgotmedia"));
        upload::upload_selected_images(upload_image_possible, path);
        upload::escape();
        sleep::ms(500);
        //图片额外需要发送一次消息
        send::send_message();
        sleep::ms(500);
    }

    //文件（交给 UploadFiles.exe：它自己找"文件"按钮、填路径并回车发送）
    if !files.is_empty() {
        log::print(localization::get(translate, "info.uploadgotfile"));
        if !upload::upload_files_by_exe(files) {
            log::warn(localization::get(translate, "error.uploadfile.failed"));
        }
        sleep::ms(500);
    }
    //音频（需要先打开"立体声混音"，否则 QQ 录不到声音）
    if !audios.is_empty() {
        log::print(localization::get(translate, "info.uploadgotaudio"));
        if let Err(error) = upload_audios(
            scale,
            record_audio_possible,
            recording_audio_possible,
            audios,
        ) {
            log::warn(format!(
                "{}: {error}",
                localization::get(translate, "error.uploadaudio.failed")
            ));
        }
    }
    sleep::ms(500);
}

/// 把音频"录"进 QQ：点开录音界面 → 按住录音键逐个阻塞播放 wav → 退出录音界面。
///
/// 之前这里套了五层 `if let`，失败分支全是同一句"模板匹配失败 + 按 esc"；
/// 现在拆成两个小函数，错误往上抛，收尾只有一个地方做。
fn upload_audios(
    scale: f32,
    record_audio_possible: RectI,
    recording_audio_possible: RectI,
    audios: &[String],
) -> Result<(), String> {
    // 1. 找"录音"按钮 —— 这一步失败时还没进录音界面，不需要按 esc
    vision::screenshot(record_audio_possible);
    sleep::ms(500);

    let (record_x, record_y) =
        vision::find_template_point("./record.png", record_audio_possible, true)?;
    log::print(format!("({record_x},{record_y})"));
    // 2. 点开录音界面
    gui_operation::click(record_x, record_y);
    sleep::ms(200);

    // 3. 界面里的操作；无论成功失败都要离开这个界面
    let result = record_all_audios(scale, recording_audio_possible, audios);
    if result.is_ok() {
        // 正常路径：再点一次"录音"按钮退出，然后补一个 esc
        gui_operation::click(record_x, record_y);
        sleep::ms(200);
    }
    // 失败时界面可能停在半路，按 esc 兜底（正常路径也保留这个习惯）
    press_key("esc");
    result
}

/// [`upload_audios`] 里"已经在录音界面中"的那一步：找按住说话的按钮，逐个播完。
fn record_all_audios(
    scale: f32,
    recording_audio_possible: RectI,
    audios: &[String],
) -> Result<(), String> {
    vision::screenshot(recording_audio_possible);
    sleep::ms(500);

    // 这个模板不要求坐标非零：录音界面铺满整块区域，(0,0) 也可能真的是它
    let (press_x, press_y) =
        vision::find_template_point("./recording.png", recording_audio_possible, false)?;

    // 模板左上角点下去会偏，往右下挪 30 个逻辑像素才落在"按住说话"中间
    let press_x = (press_x as f32 + 30.0 * scale) as i32;
    let press_y = (press_y as f32 + 30.0 * scale) as i32;
    log::print(format!("({press_x},{press_y})"));

    for path in audios {
        // 按住录音键 -> 阻塞播放（播完才松手）-> 松手
        gui_operation::mouse_down(press_x, press_y);
        sleep::ms(1000);
        let played = audio::play_blocking(path);
        gui_operation::mouse_up(press_x, press_y);
        sleep::ms(200);

        if !played {
            return Err(format!("播放失败: {path}"));
        }
    }
    Ok(())
}
mod send;

/// 从 exe 目录下的 `Images` 里挑一张图上传。
fn upload_image_without_send(upload_image_possible: RectI) {
    let translate = localization::load();
    let image_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|dir| dir.join("Images")))
        .unwrap_or_else(|| PathBuf::from("Images"));
    // log::debug(format!("image dir:{}",image_dir.));
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
        log::error(localization::get(&translate, "error.imagefoldernotfound"));
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

#[allow(clippy::too_many_arguments)]
/// 对应 `Program.GoBack`：确保已经退出会话。
fn go_back(
    settings: &config::Config,
    chat_button: PointI,
    _contact_button: PointI,
    copy_button_possible: RectI,
    upload_image_possible: RectI,
    recording_audio_possible: RectI,
) {
    let mut count = 0;
    loop {
        gui_operation::click(
            chat_button.0 + (100.0 * settings.scale) as i32,
            chat_button.1 + (80.0 * settings.scale) as i32,
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
        if settings.remote_audios {

            vision::screenshot(recording_audio_possible);
            let points = vision::find_templates(vision::SCREENSHOT_FILE, "./recording.png", 30, 1)
                .unwrap_or(vec![]);
            if !points.is_empty() {
                log::print(format!("({},{})", points[0].0, points[0].1));
                continue;
            }
        }
        vision::screenshot(copy_button_possible);
        sleep::ms(1500);
        let copy_points =
            vision::find_templates(vision::SCREENSHOT_FILE, "./copy.png", 30, 1).unwrap_or(vec![]);
        if !copy_points.is_empty() {
            log::print(format!("({},{})", copy_points[0].0, copy_points[0].1));
            continue;
        }

        break;
    }
    ms_output((settings.sleep * 1000) as u64);
}
