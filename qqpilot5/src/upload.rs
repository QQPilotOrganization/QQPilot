//! 图片上传（对应 C# 的 `upload.cs` + `Upload2.cs`）。

use std::process::Command;
use std::thread;
use std::time::Duration;

use crate::gui;
use crate::log;
use crate::native;
use crate::positions::RectI;
use crate::vision;

/// 对应 `Upload.upload()`：从 `./Images` 随机挑一张图上传。
pub fn upload() -> i32 {
    native::upload_file().upload()
}

/// 对应 `Upload.escape()`：关掉"请选择"对话框。
pub fn escape() {
    native::upload_file().escape();
}

/// 对应 `Upload2.UploadImage`：先找上传按钮，找不到就退回 `uploadImage2.exe`。
pub fn upload_image(upload_image_possible_actual_size: RectI) {
    vision::screenshot(upload_image_possible_actual_size);
    thread::sleep(Duration::from_millis(2000));

    let found = vision::find_templates(vision::SCREENSHOT_FILE, "uploadImage.png", 30, 1)
        .expect("模板匹配失败");

    if found.is_empty() {
        log::print("使用模板匹配查找上传图片按钮失败");
        let _ = Command::new("uploadImage2.exe").status();
        thread::sleep(Duration::from_millis(200));
        gui::hot_key("ctrl", "v");
    } else {
        let (x, y) = found[0];
        gui::click(
            (x + upload_image_possible_actual_size.0 as u32) as i32,
            (y + upload_image_possible_actual_size.1 as u32) as i32,
        );
        thread::sleep(Duration::from_millis(4000));
        upload();
    }
    thread::sleep(Duration::from_millis(4000));
}

/// 对应 `Upload2.UploadSelectedImage`：上传指定文件。
pub fn upload_selected_image(upload_image_possible_actual_size: RectI, file: &str) {
    vision::screenshot(upload_image_possible_actual_size);
    thread::sleep(Duration::from_millis(2000));

    let found = vision::find_templates(vision::SCREENSHOT_FILE, "uploadImage.png", 30, 1)
        .expect("模板匹配失败");

    if found.is_empty() {
        log::print("使用模板匹配查找上传图片按钮失败");
        let _ = Command::new("uploadImage2.exe").status();
        thread::sleep(Duration::from_millis(200));
        gui::hot_key("ctrl", "v");
    } else {
        let (x, y) = found[0];
        gui::click(
            (x + upload_image_possible_actual_size.0 as u32) as i32,
            (y + upload_image_possible_actual_size.1 as u32) as i32,
        );
        thread::sleep(Duration::from_millis(4000));
        native::upload_file().upload_selected_image(file);
    }
    thread::sleep(Duration::from_millis(4000));
}
