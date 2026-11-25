//! 音频播放（对应 `VisionQQ_C/audio` 编译出的 `audio.dll`）。
//!
//! DLL 只有一个导出：`bool playSound2(const char* file)` —— 底层是 miniaudio，
//! **阻塞式**：启动播放后一直等到声音放完才返回，返回值表示"有没有放成功"。
//!
//! 支持的文件格式由 miniaudio 决定（wav / mp3 / flac 常见格式都行），
//! 路径用普通字符串即可，内部会转成 UTF-8 的 C 字符串传给 DLL。

use crate::native;

/// 阻塞式播放一个音频文件，**播完才返回**。
///
/// * 成功（完整播放）返回 `true`
/// * 文件不存在、格式不支持、音频设备打不开都返回 `false`
///
/// 注意：调用线程会被占住整段播放时间。需要边跑流程边放声音的话，
/// 调用方自己 `std::thread::spawn` 一下即可（本模块只提供阻塞版本）。
#[allow(dead_code)] // 预留给调用方（例如提示音），暂时还没有地方用它
pub fn play_blocking(file: &str) -> bool {
    native::audio_dll().play_sound(file)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::*;

    /// 写一段静音 WAV（16bit 单声道），用来验证"真的播放并且阻塞到结束"。
    fn write_silent_wav(path: &Path, seconds: f32) -> std::io::Result<()> {
        const SAMPLE_RATE: u32 = 44_100;
        let samples = (SAMPLE_RATE as f32 * seconds) as u32;
        let data_size = samples * 2;

        let mut bytes = Vec::with_capacity(44 + data_size as usize);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(36 + data_size).to_le_bytes());
        bytes.extend_from_slice(b"WAVE");
        bytes.extend_from_slice(b"fmt ");
        bytes.extend_from_slice(&16u32.to_le_bytes()); // fmt 块长度
        bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
        bytes.extend_from_slice(&1u16.to_le_bytes()); // 单声道
        bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
        bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // 每秒字节数
        bytes.extend_from_slice(&2u16.to_le_bytes()); // 块对齐
        bytes.extend_from_slice(&16u16.to_le_bytes()); // 位深
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_size.to_le_bytes());
        bytes.resize(44 + data_size as usize, 0);
        fs::write(path, bytes)
    }

    /// 没有 `audio.dll` 时跳过（和 native.rs 里那几条一样）。
    fn dll_missing() -> bool {
        crate::native::find_dll("audio.dll").is_err()
    }

    /// 播放一段 0.3 秒的静音 WAV：必须放完才返回。
    ///
    /// 静音是为了跑测试时不会突然出声；时长断言用来证明它确实是**阻塞**的。
    #[test]
    fn plays_a_wav_and_blocks_until_finished() {
        if dll_missing() {
            eprintln!("跳过：当前目录没有 audio.dll");
            return;
        }

        let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("test-tmp");
        fs::create_dir_all(&dir).expect("创建临时目录失败");
        let wav = dir.join("audio-test-silent.wav");
        write_silent_wav(&wav, 0.3).expect("写 WAV 失败");
        let wav = wav.to_string_lossy().into_owned();

        let start = Instant::now();
        let ok = play_blocking(&wav);
        let elapsed = start.elapsed();
        let _ = fs::remove_file(&wav);

        assert!(ok, "播放失败：{wav}");
        assert!(
            elapsed >= Duration::from_millis(250),
            "没有阻塞到播完，只用了 {elapsed:?}"
        );
    }

    /// 文件不存在要老老实实返回 false，而不是崩掉。
    #[test]
    fn missing_file_returns_false() {
        if dll_missing() {
            eprintln!("跳过：当前目录没有 audio.dll");
            return;
        }
        assert!(!play_blocking("这个文件不存在.wav"));
    }
}
