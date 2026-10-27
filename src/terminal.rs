//! External consoles never enter Tether's managed SSH jobs or reconnect loop.
use crate::{connection_command::ConnectionCommand, environments::Environment, model::Result};
use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};

const PAYLOAD_ENV: &str = "TETHER_TERMINAL_COMMAND";

#[derive(Serialize, Deserialize)]
struct Launch {
    program: String,
    args: Vec<String>,
    distro: Option<String>,
    user: Option<String>,
}

pub fn run() -> Result<i32> {
    let launch: Launch =
        serde_json::from_str(&std::env::var(PAYLOAD_ENV).map_err(|_| "缺少终端启动参数")?)
            .map_err(|e| e.to_string())?;
    let mut command = if let Some(distro) = launch.distro {
        let mut command = Command::new("wsl.exe");
        command.args([
            "--distribution",
            &distro,
            "--user",
            launch.user.as_deref().ok_or("缺少 WSL 用户")?,
            "--exec",
            &launch.program,
        ]);
        command
    } else {
        Command::new(&launch.program)
    };
    // Preserve structured argv and console IO. The terminal handles all SSH prompts.
    let status = command
        .args(&launch.args)
        .env_remove(PAYLOAD_ENV)
        .env_remove("TETHER_ASKPASS_URL")
        .env_remove("TETHER_ASKPASS_TOKEN")
        .env_remove("TETHER_ASKPASS_GROUP")
        .env_remove("SSH_ASKPASS")
        .env_remove("SSH_ASKPASS_REQUIRE")
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("无法启动外部 SSH：{e}"))?;
    let code = status.code().unwrap_or(1);
    println!("\nSSH 已结束，退出码 {code}。可继续使用此终端。");
    Ok(code)
}

#[cfg(windows)]
pub fn open(command: ConnectionCommand, environment: Option<&Environment>) -> Result<u32> {
    use std::{mem::size_of, os::windows::ffi::OsStrExt};
    use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::*};
    let launch = Launch {
        program: command.program,
        args: command.args,
        distro: environment.map(|e| e.distro.clone()),
        user: environment.map(|e| e.user.clone()),
    };
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let invocation = format!("try {{ & '{}' terminal-run }} finally {{ Remove-Item Env:{PAYLOAD_ENV} -ErrorAction SilentlyContinue }}", exe.to_string_lossy().replace('\'', "''"));
    let mut line: Vec<u16> = format!("powershell.exe -NoProfile -NoExit -Command {invocation}")
        .encode_utf16()
        .chain([0])
        .collect();
    let mut env: Vec<_> = std::env::vars_os()
        .filter(|(k, _)| !k.to_string_lossy().eq_ignore_ascii_case(PAYLOAD_ENV))
        .collect();
    env.push((
        PAYLOAD_ENV.into(),
        serde_json::to_string(&launch)
            .map_err(|e| e.to_string())?
            .into(),
    ));
    env.sort_by_key(|(key, _)| key.to_string_lossy().to_uppercase());
    let mut block = Vec::<u16>::new();
    for (key, value) in env {
        block.extend(key.encode_wide());
        block.push(b'=' as u16);
        block.extend(value.encode_wide());
        block.push(0);
    }
    block.push(0);
    // Unset STARTF_USESTDHANDLES gives the new console its own IO instead of
    // inheriting the backend's authenticated stdio/bridge pipes.
    unsafe {
        let startup = STARTUPINFOW {
            cb: size_of::<STARTUPINFOW>() as u32,
            ..std::mem::zeroed()
        };
        let mut process: PROCESS_INFORMATION = std::mem::zeroed();
        if CreateProcessW(
            std::ptr::null(),
            line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_NEW_CONSOLE | CREATE_UNICODE_ENVIRONMENT,
            block.as_ptr().cast(),
            std::ptr::null(),
            &startup,
            &mut process,
        ) == 0
        {
            return Err(format!(
                "无法打开外部 PowerShell 终端：{}",
                std::io::Error::last_os_error()
            ));
        }
        CloseHandle(process.hThread);
        CloseHandle(process.hProcess);
        Ok(process.dwProcessId)
    }
}

#[cfg(not(windows))]
pub fn open(_: ConnectionCommand, _: Option<&Environment>) -> Result<u32> {
    Err("请从 Windows Tether 页面打开外部终端；独立 Linux 后台可使用复制命令".into())
}
