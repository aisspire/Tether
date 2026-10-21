//! File and identity operations stay in Tether, never in the workspace host.
use crate::{model::*, runtime::Runtime, ssh, store};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
fn path_arg(v: &Value) -> Result<PathBuf> {
    let p = PathBuf::from(v["path"].as_str().ok_or("缺少文件路径")?);
    if !p.is_absolute() {
        return Err("文件路径须为绝对路径".into());
    }
    Ok(p)
}
fn registered(rt: &Runtime, p: &Path, keys: bool) -> bool {
    let s = rt.store.lock().unwrap();
    let c = &s.config;
    if keys {
        c.key_files.iter().any(|x| Path::new(x) == p)
    } else {
        c.ssh_files.iter().any(|x| Path::new(x) == p)
            || c.hosts
                .iter()
                .any(|h| h.config_file.as_deref().is_some_and(|x| Path::new(x) == p))
    }
}
fn tool(program: &str) -> tokio::process::Command {
    ssh::command(program)
}
fn helper_env(rt: &Runtime, cmd: &mut tokio::process::Command) {
    cmd.env("SSH_ASKPASS", std::env::current_exe().unwrap())
        .env("SSH_ASKPASS_REQUIRE", "force")
        .env("DISPLAY", "tether:0")
        .env("TETHER_ASKPASS_URL", &rt.auth_origin)
        .env("TETHER_ASKPASS_TOKEN", &rt.auth_token)
        .env("TETHER_ASKPASS_GROUP", "tools");
}
async fn interactive(rt: &Runtime, cmd: tokio::process::Command) -> Result<String> {
    let mut cancelled = rt.tool_cancel.subscribe();
    tokio::select! {result=ssh::output(cmd,120)=>result,_=cancelled.changed()=>Err("密钥或 Agent 操作已取消".into())}
}
pub async fn action(rt: &Arc<Runtime>, v: Value) -> Result<Value> {
    match v["action"].as_str().unwrap_or("") {
        "files.read" => {
            let p = path_arg(&v)?;
            if !registered(rt, &p, false) {
                return Err("请先在配置中登记 SSH 文本文件".into());
            }
            let bytes = store::read(&p)?;
            let text = String::from_utf8(bytes.clone())
                .map_err(|_| "文件不是 UTF-8，保留原文件，请使用外部编辑器")?;
            Ok(json!({"path":p,"text":text,"revision":store::revision(&bytes)}))
        }
        "files.write" => {
            let p = path_arg(&v)?;
            if !registered(rt, &p, false) {
                return Err("文件未登记".into());
            }
            let old = store::read(&p)?;
            if Some(store::revision(&old).as_str()) != v["revision"].as_str() {
                return Err("原文件已被外部修改；请重新加载".into());
            }
            let text = v["text"].as_str().ok_or("缺少文本")?;
            if text.len() > 2 * 1024 * 1024 {
                return Err("文件过大".into());
            }
            let hosts = rt
                .store
                .lock()
                .unwrap()
                .config
                .hosts
                .iter()
                .filter(|h| h.config_file.as_deref().is_some_and(|f| Path::new(f) == p))
                .cloned()
                .collect::<Vec<_>>();
            for host in hosts {
                let pending = p.with_file_name(format!(".tether-check-{}.conf", id()));
                store::atomic(&pending, text.as_bytes())?;
                let mut cmd = tool(&host.ssh);
                cmd.args(["-G", "-F"]).arg(&pending).arg(&host.destination);
                let checked = ssh::output(cmd, host.timeout_secs).await;
                let _ = std::fs::remove_file(pending);
                checked.map_err(|e| format!("SSH 配置校验失败，未保存：{e}"))?;
            }
            if store::read(&p)? != old {
                return Err("校验期间原文件被外部修改，请重新加载".into());
            }
            let backup = p.with_file_name(format!(
                "{}.tether-{}.bak",
                p.file_name().unwrap().to_string_lossy(),
                id()
            ));
            std::fs::copy(&p, &backup).map_err(|e| e.to_string())?;
            let permissions = std::fs::metadata(&p)
                .map_err(|e| e.to_string())?
                .permissions();
            store::atomic(&p, text.as_bytes())?;
            std::fs::set_permissions(&p, permissions)
                .map_err(|e| format!("已写入但恢复权限失败：{e}"))?;
            Ok(json!({"saved":true,"backup":backup,"revision":store::revision(text.as_bytes())}))
        }
        "keys.list" => {
            let config = rt.store.lock().unwrap().config.clone();
            let mut keys = vec![];
            for p in &config.key_files {
                let path = PathBuf::from(p);
                let public = if p.ends_with(".pub") {
                    path.clone()
                } else {
                    PathBuf::from(format!("{p}.pub"))
                };
                let mut cmd = tool("ssh-keygen");
                cmd.args(["-lf"]).arg(&public).args(["-E", "sha256"]);
                let fingerprint = ssh::output(cmd, 5).await;
                keys.push(json!({"path":p,"publicPath":public,"privateFile":!p.ends_with(".pub")&&path.exists(),"publicFile":public.exists(),"fingerprint":fingerprint.as_ref().ok(),"error":fingerprint.as_ref().err()}));
            }
            let mut cmd = tool("ssh-add");
            cmd.arg("-L");
            let agent = ssh::output(cmd, 5).await;
            Ok(json!({"files":keys,"agent":agent.as_ref().ok(),"agentError":agent.as_ref().err()}))
        }
        "keys.public" => {
            let p = path_arg(&v)?;
            if !registered(rt, &p, true) {
                return Err("密钥未登记".into());
            }
            let public = if p.to_string_lossy().ends_with(".pub") {
                p
            } else {
                PathBuf::from(format!("{}.pub", p.to_string_lossy()))
            };
            let bytes = store::read(&public)?;
            let text = String::from_utf8(bytes).map_err(|_| "公钥编码无效")?;
            if !["ssh-", "ecdsa-", "sk-"]
                .iter()
                .any(|prefix| text.starts_with(prefix))
            {
                return Err("文件不是可辨识的公钥".into());
            }
            Ok(json!({"publicKey":text}))
        }
        "keys.generate" => {
            let p = path_arg(&v)?;
            if !registered(rt, &p, true) {
                return Err("请先登记新密钥的保存路径".into());
            }
            if p.exists() || PathBuf::from(format!("{}.pub", p.to_string_lossy())).exists() {
                return Err("密钥文件已存在，不覆盖".into());
            }
            let alg = v["algorithm"].as_str().unwrap_or("ed25519");
            if !["ed25519", "rsa", "ecdsa"].contains(&alg) {
                return Err("不支持的密钥算法".into());
            }
            let mut cmd = tool("ssh-keygen");
            cmd.args(["-q", "-t", alg, "-f"])
                .arg(&p)
                .args(["-C", "Tether"]);
            helper_env(rt, &mut cmd);
            interactive(rt, cmd).await?;
            Ok(json!({"generated":true,"path":p}))
        }
        "keys.passphrase" | "keys.comment" => {
            let p = path_arg(&v)?;
            if !registered(rt, &p, true) || p.to_string_lossy().ends_with(".pub") {
                return Err("请登记实际私钥文件，公钥或 Agent-only 身份无法修改私钥".into());
            }
            let mut cmd = tool("ssh-keygen");
            if v["action"] == "keys.passphrase" {
                cmd.arg("-p");
            } else {
                cmd.args(["-c", "-C", v["comment"].as_str().ok_or("缺少公钥备注")?]);
            }
            cmd.arg("-f").arg(p);
            helper_env(rt, &mut cmd);
            Ok(json!({"result":ssh::output(cmd,120).await?}))
        }
        "agent.add" | "agent.remove" => {
            let p = path_arg(&v)?;
            if !registered(rt, &p, true) {
                return Err("密钥未登记".into());
            }
            let mut cmd = tool("ssh-add");
            if v["action"] == "agent.remove" {
                cmd.arg("-d");
            }
            cmd.arg(p);
            helper_env(rt, &mut cmd);
            let result = interactive(rt, cmd).await?;
            Ok(json!({"result":result}))
        }
        "knownHosts.find" | "knownHosts.remove" => {
            let p = path_arg(&v)?;
            if !registered(rt, &p, false) {
                return Err("请先登记 known_hosts 文件".into());
            }
            let host = v["host"]
                .as_str()
                .filter(|s| !s.starts_with('-') && !s.contains(['\n', '\r']))
                .ok_or("缺少有效主机")?;
            if v["action"] == "knownHosts.remove" && v["confirm"] != true {
                return Err("删除主机指纹需要明确确认".into());
            }
            let mut cmd = tool("ssh-keygen");
            cmd.args([
                if v["action"] == "knownHosts.remove" {
                    "-R"
                } else {
                    "-F"
                },
                host,
                "-f",
            ])
            .arg(p);
            Ok(json!({"result":ssh::output(cmd,5).await?}))
        }
        "observe" => observe(rt).await,
        _ => Err("未知工具动作".into()),
    }
}
async fn observe(rt: &Runtime) -> Result<Value> {
    #[cfg(windows)]
    {
        let mut cmd = tool("powershell.exe");
        cmd.args(["-NoProfile","-NonInteractive","-Command", "@(Get-CimInstance Win32_Process -Filter \"Name='ssh.exe'\" | Select-Object ProcessId,ParentProcessId,CreationDate,ExecutablePath,CommandLine) | ConvertTo-Json -Compress"]);
        let out = ssh::output(cmd, 10).await?;
        let mut data = serde_json::from_str::<Value>(&out).unwrap_or(json!([]));
        let owned = rt.owned_pids();
        if let Some(processes) = data.as_array_mut() {
            for process in processes {
                process["source"] = json!(if process["ProcessId"]
                    .as_u64()
                    .is_some_and(|id| owned.contains(&(id as u32)))
                {
                    "tether"
                } else {
                    "external"
                });
            }
        }
        Ok(
            json!({"readOnly":true,"processes":data,"scope":"可读取的 ssh.exe 进程命令行；不推断配置继承或远程监听"}),
        )
    }
    #[cfg(not(windows))]
    {
        let _ = rt;
        Ok(
            json!({"readOnly":true,"processes":[],"scope":"外部观测仅在 Windows 实现；不推断未知进程"}),
        )
    }
}
