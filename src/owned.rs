//! Live handles, not PID discovery, are the authority for cleanup.
use crate::{model::Result, ssh};
use std::process::Stdio;
use tokio::process::Child;
pub struct Owned {
    pub child: Child,
    #[cfg(windows)]
    job: Job,
}
#[cfg(windows)]
struct Job(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
unsafe impl Send for Job {}
#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}
impl Owned {
    pub fn spawn(
        program: &str,
        args: &[String],
        env: &[(String, String)],
        hidden: bool,
    ) -> Result<Self> {
        let mut cmd = ssh::command(program);
        cmd.args(args)
            .envs(env.iter().cloned())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            cmd.process_group(0);
        }
        #[cfg(windows)]
        {
            cmd.creation_flags(0x00000004 | if hidden { 0x08000000 } else { 0x00000010 });
        }
        #[cfg(not(windows))]
        let _ = hidden;
        #[allow(unused_mut)]
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        #[cfg(windows)]
        {
            use windows_sys::Win32::{Foundation::*, System::JobObjects::*};
            #[link(name = "ntdll")]
            unsafe extern "system" {
                fn NtResumeProcess(handle: HANDLE) -> i32;
            }
            let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if job.is_null() {
                let _ = child.start_kill();
                return Err("无法创建 SSH Job Object".into());
            }
            let job = Job(job);
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let raw = child.raw_handle().ok_or("SSH 进程句柄不可用")? as HANDLE;
            if unsafe {
                SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const _,
                    std::mem::size_of_val(&info) as u32,
                )
            } == 0
                || unsafe { AssignProcessToJobObject(job.0, raw) } == 0
                || unsafe { NtResumeProcess(raw) } < 0
            {
                let _ = child.start_kill();
                return Err("SSH 归属或恢复失败，已取消启动".into());
            }
            return Ok(Self { child, job });
        }
        #[cfg(not(windows))]
        Ok(Self { child })
    }
    pub async fn stop(&mut self) {
        #[cfg(windows)]
        unsafe {
            windows_sys::Win32::System::JobObjects::TerminateJobObject(self.job.0, 1);
        }
        #[cfg(unix)]
        if let Some(pid) = self.child.id() {
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
        let _ = self.child.start_kill();
        let _ = self.child.wait().await;
    }
}
