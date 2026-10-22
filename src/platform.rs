//! OS boundaries. Business actions never translate paths or switch process-wide environments.
use crate::{model::Result, store};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;

pub fn command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.kill_on_drop(true).stdin(Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(0x08000000);
    cmd
}
pub fn local_id() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else {
        "linux"
    }
}
pub fn local_label() -> String {
    if cfg!(windows) {
        "Windows".into()
    } else {
        std::env::var("WSL_DISTRO_NAME")
            .map(|s| format!("WSL · {s}"))
            .unwrap_or("Linux".into())
    }
}
pub fn runtime_dir(config: &Path) -> Result<PathBuf> {
    let canonical = std::fs::canonicalize(config).map_err(|e| e.to_string())?;
    let hash = store::revision(canonical.to_string_lossy().as_bytes());
    #[cfg(windows)]
    let base = PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("缺少 LOCALAPPDATA")?)
        .join("Tether/run");
    #[cfg(unix)]
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("tether-{}", unsafe { libc::geteuid() }))
        })
        .join("tether");
    store::private_dir(&base)?;
    let dir = base.join(&hash[..32]);
    store::private_dir(&dir)?;
    Ok(dir)
}
pub fn decode(bytes: &[u8]) -> String {
    if bytes.contains(&0) {
        String::from_utf16_lossy(
            &bytes
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect::<Vec<_>>(),
        )
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
    .trim_start_matches('\u{feff}')
    .trim()
    .to_owned()
}
pub async fn wsl(distro: &str, args: &[&str]) -> Result<String> {
    let mut cmd = command("wsl.exe");
    cmd.args(["--distribution", distro, "--exec"]).args(args);
    let out = tokio::time::timeout(std::time::Duration::from_secs(20), cmd.output())
        .await
        .map_err(|_| "WSL 启动或命令超时")?
        .map_err(|e| format!("无法运行 WSL：{e}"))?;
    if !out.status.success() {
        return Err(format!(
            "WSL {distro}：{} {}",
            decode(&out.stderr),
            decode(&out.stdout)
        ));
    }
    Ok(decode(&out.stdout))
}
pub async fn wsl_path(distro: &str, path: &Path) -> Result<String> {
    wsl(distro, &["wslpath", "-a", "-u", &path.to_string_lossy()]).await
}
pub async fn choose_file(initial: &str) -> Result<Option<String>> {
    #[cfg(windows)]
    {
        let mut cmd = command("powershell.exe");
        cmd.env("TETHER_PICK_INITIAL", initial).args(["-NoProfile", "-STA", "-Command", r#"Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.OpenFileDialog; $d.Title = 'Tether — SSH file'; $d.CheckFileExists = $true; if ($env:TETHER_PICK_INITIAL) { $d.InitialDirectory = $env:TETHER_PICK_INITIAL }; if ($d.ShowDialog() -eq 'OK') { [Console]::OutputEncoding = [Text.UTF8Encoding]::new($false); [Console]::Write($d.FileName) }"#]);
        let out = cmd
            .output()
            .await
            .map_err(|e| format!("文件选择器无法打开：{e}"))?;
        if !out.status.success() {
            return Err(decode(&out.stderr));
        }
        let path = decode(&out.stdout);
        Ok((!path.is_empty()).then_some(path))
    }
    #[cfg(not(windows))]
    {
        let _ = initial;
        Err("独立 Linux 后台请填写文件绝对路径；Windows 入口支持 WSL 文件选择".into())
    }
}
