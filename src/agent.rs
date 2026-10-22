#[cfg(unix)]
use crate::platform;
use crate::{model::*, runtime::Runtime};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(unix)]
async fn valid(path: &Path) -> bool {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let Ok(m) = std::fs::metadata(path) else {
        return false;
    };
    if !m.file_type().is_socket() || m.uid() != unsafe { libc::geteuid() } {
        return false;
    }
    let mut cmd = platform::command("ssh-add");
    cmd.arg("-l").env("SSH_AUTH_SOCK", path);
    match tokio::time::timeout(std::time::Duration::from_secs(2), cmd.output()).await {
        Ok(Ok(out)) => {
            out.status.success()
                || out.status.code() == Some(1)
                    && String::from_utf8_lossy(&out.stdout).contains("no identities")
        }
        _ => false,
    }
}
#[cfg(not(unix))]
async fn valid(_path: &Path) -> bool {
    false
}

pub async fn prepare(rt: &Runtime) -> Value {
    let _gate = rt.agent_gate.lock().await;
    if cfg!(windows) {
        return json!({"available":true,"candidates":[],"socket":null});
    }
    let configured = rt.store.lock().unwrap().config.agent_socket.clone();
    let inherited = std::env::var("SSH_AUTH_SOCK").ok();
    let previous = rt.agent_socket.lock().unwrap().clone();
    let mut paths = Vec::<PathBuf>::new();
    for p in [configured.as_ref(), inherited.as_ref(), previous.as_ref()]
        .into_iter()
        .flatten()
    {
        paths.push(PathBuf::from(p));
    }
    if let Ok(dirs) = std::fs::read_dir(std::env::temp_dir()) {
        for dir in dirs
            .flatten()
            .filter(|d| d.file_name().to_string_lossy().starts_with("ssh-"))
        {
            if let Ok(files) = std::fs::read_dir(dir.path()) {
                for f in files
                    .flatten()
                    .filter(|f| f.file_name().to_string_lossy().starts_with("agent."))
                {
                    paths.push(f.path());
                }
            }
        }
    }
    #[cfg(unix)]
    for p in [
        format!("/run/user/{}/ssh-agent.socket", unsafe { libc::geteuid() }),
        format!("/run/user/{}/gcr/ssh", unsafe { libc::geteuid() }),
    ] {
        paths.push(p.into());
    }
    paths.sort();
    paths.dedup();
    let mut candidates = vec![];
    for p in paths {
        if valid(&p).await {
            candidates.push(p.to_string_lossy().into_owned());
        }
    }
    let preferred = [configured.as_ref(), inherited.as_ref(), previous.as_ref()]
        .into_iter()
        .flatten()
        .find(|p| candidates.contains(p))
        .cloned();
    let socket = if configured.is_some() {
        configured.filter(|p| candidates.contains(p))
    } else {
        preferred.or_else(|| (candidates.len() == 1).then(|| candidates[0].clone()))
    };
    *rt.agent_socket.lock().unwrap() = socket.clone();
    json!({"available":socket.is_some(),"socket":socket,"candidates":candidates,"reason":if socket.is_some(){""}else{"未找到有效的 Agent 连接或需要选择 socket；请绑定已有 Agent，不会自动启动服务"}})
}

pub fn apply(rt: &Runtime, cmd: &mut tokio::process::Command) {
    if cfg!(unix) {
        cmd.env_remove("SSH_AUTH_SOCK");
        if let Some(p) = rt.agent_socket.lock().unwrap().as_ref() {
            cmd.env("SSH_AUTH_SOCK", p);
        }
    }
}
pub async fn action(rt: &Arc<Runtime>, v: Value) -> Result<Value> {
    if v["action"] == "agent.bind" {
        let path = v["path"].as_str().ok_or("缺少 Agent socket 路径")?;
        if !Path::new(path).is_absolute() || !valid(Path::new(path)).await {
            return Err("该 socket 不属于当前用户、无法访问或不是有效 Agent".into());
        }
        let mut s = rt.store.lock().unwrap();
        let mut c = s.config.clone();
        c.agent_socket = Some(path.into());
        let rev = s.revision.clone();
        s.save(c, &rev)?;
    }
    Ok(prepare(rt).await)
}
