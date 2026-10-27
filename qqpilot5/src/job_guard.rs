use std::process::Command;

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};

use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};

use windows_sys::core::PCWSTR;

use std::ptr;

pub struct JobGuard {
    pub(crate) handle: windows_sys::Win32::Foundation::HANDLE,
}

pub(crate) fn str_to_pcwstr(s: &str) -> Vec<u16> {
    
    s
        .to_string()
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect()
}

impl JobGuard {
    pub fn new() -> Result<Self, String> {
        // 名字缓冲区必须活到 CreateJobObjectW 返回；写在调用里会随临时值释放。
        let name = str_to_pcwstr("RunningQQPilotInfo");
        let handle = unsafe { CreateJobObjectW(ptr::null(), PCWSTR::from(name.as_ptr())) };
        if handle.is_null() {
            let translate = localization::load();
            return Err(localization::get(&translate, "error.jobobjects.create"));
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let success = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        log::debug(format!("success:{}", success));
        if success == 0 {
            let error = unsafe { GetLastError() };
            unsafe { CloseHandle(handle) };
            let translate = localization::load();
            log::error(format!("{}", error));
            return Err(localization::get(&translate, "error.jobobjects.limit"));
        }

        Ok(Self { handle })
    }
    pub fn assign_process(&self, child: &std::process::Child) -> Result<(), String> {
        unsafe {
            let pid = child.id();
            // 使用 OpenProcess 获取进程句柄。
            // AssignProcessToJobObject 要求句柄同时具备这两个权限，
            // 只给 PROCESS_SET_QUOTA 会失败并返回 ERROR_ACCESS_DENIED(5)。
            use windows_sys::Win32::System::Threading::{
                OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE,
            };
            let proc_handle = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, 0, pid);
            if proc_handle.is_null() {
                let translate = localization::load();

                return Err(localization::get(
                    &translate,
                    "error.jobobjects.subprocess.handle",
                ));
            }

            let result = AssignProcessToJobObject(self.handle, proc_handle);
            let error = GetLastError();
            CloseHandle(proc_handle); // 用完立即关闭临时句柄

            if result == 0 {
                let translate = localization::load();
                log::error(format!("{}", error));
                return Err(localization::get(&translate, "error.jobobjects.attach"));
            }
        }
        Ok(())
    }
    /// 拉起一个子进程并把它放进 Job：返回的 guard 一释放，子进程就跟着结束。
    pub fn create_and_assign(program: &std::path::Path, args: &[&str]) -> Result<Self, String> {
        let translate = localization::load();
        // 先建 Job 再拉起进程：Job 建不出来时不会留下一个没人管的子进程。
        let job = JobGuard::new()?;
        let mut child = Command::new(program).args(args).spawn().map_err(|e| {
            format!(
                "{}: {}: {e}",
                localization::get(&translate, "error.subprocess.launch"),
                program.display()
            )
        })?;
        if let Err(error) = job.assign_process(&child) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
        Ok(job)
    }
}

/// 找可执行文件：先可执行文件目录，再当前工作目录；都找不到就原样返回，
/// 让 `Command::new` 按 PATH 找（也能给出更直观的错误）。
pub fn resolve_program(name: &str) -> std::path::PathBuf {
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(std::path::PathBuf::from))
    {
        candidates.push(dir.join(name));
    }
    if let Ok(dir) = std::env::current_dir() {
        candidates.push(dir.join(name));
    }
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .unwrap_or_else(|| std::path::PathBuf::from(name))
}

use crate::{localization, log};
impl Drop for JobGuard {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// 所有用例共用同一个具名 Job 对象（"RunningQQPilotInfo"），并行跑会互相
    /// 开关同一个对象的句柄（句柄一关，KILL_ON_JOB_CLOSE 就会动手），所以串起来。
    static JOB_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn lock() -> std::sync::MutexGuard<'static, ()> {
        JOB_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn create_process() {
        let _guard = lock();
        JobGuard::create_and_assign(std::path::Path::new("notepad.exe"), &[]).unwrap();
    }

    /// Job 句柄全部关闭时，已经分配进去的子进程必须一起结束。
    #[test]
    fn kills_child_when_dropped() {
        let _guard = lock();
        let job = JobGuard::new().unwrap();
        let mut child = Command::new("cmd.exe")
            .args(["/c", "ping -n 60 127.0.0.1 > nul"])
            .spawn()
            .unwrap();
        job.assign_process(&child).unwrap();
        drop(job);

        let deadline = Instant::now() + Duration::from_secs(5);
        while child.try_wait().unwrap().is_none() {
            assert!(Instant::now() < deadline, "子进程没有随 Job 关闭而结束");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
