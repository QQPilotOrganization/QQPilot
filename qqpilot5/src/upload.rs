//! 图片上传（对应 C# 的 `upload.cs` + `Upload2.cs`）。

use std::process::Command;

use std::thread;
use std::time::Duration;

use crate::gui_operation;
use crate::localization;
use crate::log;
use crate::native;
use crate::positions::RectI;
use crate::vision;
/// 对应 `Upload.upload()`：从 `./Images` 随机挑一张图上传。
pub fn upload() -> i32 {
    native::upload_file().upload()
}

/// 用 `UploadFiles.exe` 上传一批**文件**（和图片/音频走的是完全不同的路子）。
///
/// 这个 exe 自己会：截屏 → 模板匹配"文件"按钮 → 点开"请选择"对话框 →
/// 一次把所有路径填进去 → 回车发送。所以这里只要把绝对路径传过去、
/// 等它退出就行，**不要**再额外调 `send_message`。
///
/// 返回它是否成功（退出码 0）。找不到 exe 或启动失败只记日志并返回 `false`。
pub fn upload_files_by_exe(files: &[String]) -> bool {
    if files.is_empty() {
        return true;
    }

    let status = Command::new("UploadFiles.exe").args(files).status();
    match status {
        Ok(status) if status.success() => true,
        Ok(status) => {
            log::error(format!(
                "UploadFiles.exe 退出码 {}",
                status.code().unwrap_or(-1)
            ));
            false
        }
        Err(e) => {
            let translate = localization::load();
            log::error(format!(
                "{}: {e}",
                localization::get(&translate, "error.subprocess.launch")
            ));
            false
        }
    }
}
/// 对应 `Upload.escape()`：关掉"请选择"对话框。
pub fn escape() {
    native::upload_file().escape();
}

/// 先找上传按钮，找不到就退回 `uploadImage2.exe`。
pub fn upload_image(upload_image_possible_actual_size: RectI) {
    let translate = localization::load();
    vision::screenshot(upload_image_possible_actual_size);
    thread::sleep(Duration::from_millis(2000));

    let failure = localization::get(&translate, "error.matchtemplate.failed");

    let found =
        vision::find_templates(vision::SCREENSHOT_FILE, "uploadImage.png", 30, 1).expect(&failure);

    if found.is_empty() {
        log::print(failure);
        let _ = Command::new("uploadImage2.exe").status();
        thread::sleep(Duration::from_millis(200));
        gui_operation::hot_key("ctrl", "v");
    } else {
        let (x, y) = found[0];
        gui_operation::click(
            (x + upload_image_possible_actual_size.0 as u32) as i32,
            (y + upload_image_possible_actual_size.1 as u32) as i32,
        );
        thread::sleep(Duration::from_millis(4000));
        upload();
    }

    thread::sleep(Duration::from_millis(4000));
}

/// 上传指定图片。
pub fn upload_selected_images(upload_image_possible_actual_size: RectI, file: &str) {
    let translate = localization::load();
    vision::screenshot(upload_image_possible_actual_size);
    thread::sleep(Duration::from_millis(2000));

    let failure = localization::get(&translate, "error.matchtemplate.failed");
    let found =
        vision::find_templates(vision::SCREENSHOT_FILE, "uploadImage.png", 30, 1).expect(&failure);

    if found.is_empty() {
        log::print(failure);
        let _ = Command::new("uploadImage2.exe").status();
        thread::sleep(Duration::from_millis(200));
        gui_operation::hot_key("ctrl", "v");
    } else {
        let (x, y) = found[0];
        gui_operation::click(
            (x + upload_image_possible_actual_size.0 as u32) as i32,
            (y + upload_image_possible_actual_size.1 as u32) as i32,
        );
        thread::sleep(Duration::from_millis(4000));
        native::upload_file().upload_selected_image(file);
    }

    thread::sleep(Duration::from_millis(4000));
}
#[cfg(test)]
mod tests {
    use super::*;

    /// 空列表不该去启动 `UploadFiles.exe`（那样会真去点 QQ 窗口）。
    #[test]
    fn empty_file_list_is_a_noop() {
        assert!(upload_files_by_exe(&[]));
    }
}
