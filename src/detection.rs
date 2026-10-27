//! Explicit endpoint observations. These records never grant process ownership or readiness.
#[cfg(windows)]
use crate::platform;
use crate::{model::*, runtime::Runtime};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    pub rule_id: String,
    pub state: String,
    pub source: Option<String>,
    pub process: Option<String>,
    pub pid: Option<u32>,
    pub reason: String,
    pub checked_at: u64,
    pub reachable: Option<bool>,
    #[serde(skip)]
    pub rule: Rule,
    #[serde(skip)]
    pub epoch: u64,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Listener {
    pub address: String,
    pub port: u16,
    pub pid: Option<u32>,
    pub inode: String,
}
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn matches(address: &str, bind: &str) -> bool {
    if address == bind {
        return true;
    }
    match (address.parse::<IpAddr>(), bind.parse::<IpAddr>()) {
        (Ok(a), Ok(b)) => a.is_ipv4() == b.is_ipv4() && (a.is_unspecified() || b.is_unspecified()),
        (Ok(a), _) if bind == "localhost" => a.is_loopback() || a.is_unspecified(),
        (_, _) => bind.is_empty() || bind == "*",
    }
}
#[cfg(unix)]
fn proc_sockets() -> Result<Vec<(Listener, String, String, String)>> {
    let mut rows = vec![];
    for file in ["/proc/net/tcp", "/proc/net/tcp6"] {
        let text = match std::fs::read_to_string(file) {
            Ok(t) => t,
            Err(e) if file.ends_with('6') && e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(format!("无法查询本机 TCP：{e}")),
        };
        for line in text.lines().skip(1) {
            let cols = line.split_whitespace().collect::<Vec<_>>();
            if cols.len() < 10 {
                continue;
            }
            let Some((address, port)) = proc_endpoint(cols[1]) else {
                continue;
            };
            rows.push((
                Listener {
                    address,
                    port,
                    pid: None,
                    inode: cols[9].into(),
                },
                cols[3].into(),
                cols[1].into(),
                cols[2].into(),
            ));
        }
    }
    Ok(rows)
}
#[cfg(unix)]
fn proc_endpoint(text: &str) -> Option<(String, u16)> {
    let (addr, port) = text.split_once(':')?;
    let port = u16::from_str_radix(port, 16).ok()?;
    let mut bytes = vec![];
    for word in addr.as_bytes().chunks_exact(8) {
        let word = std::str::from_utf8(word).ok()?;
        bytes.extend_from_slice(&u32::from_str_radix(word, 16).ok()?.to_ne_bytes());
    }
    let address = if bytes.len() == 4 {
        std::net::Ipv4Addr::from(<[u8; 4]>::try_from(bytes).ok()?).to_string()
    } else {
        std::net::Ipv6Addr::from(<[u8; 16]>::try_from(bytes).ok()?).to_string()
    };
    Some((address, port))
}
pub async fn listeners() -> Result<Vec<Listener>> {
    #[cfg(unix)]
    {
        Ok(proc_sockets()?
            .into_iter()
            .filter(|(_, s, _, _)| s == "0A")
            .map(|(l, _, _, _)| l)
            .collect())
    }
    #[cfg(windows)]
    {
        let mut cmd = platform::command("powershell.exe");
        cmd.args(["-NoProfile","-NonInteractive","-Command",r#"$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); ConvertTo-Json -Compress -InputObject @(Get-NetTCPConnection -State Listen | Select-Object @{n='address';e={$_.LocalAddress}},@{n='port';e={$_.LocalPort}},@{n='pid';e={$_.OwningProcess}},@{n='inode';e={''}})"#]);
        serde_json::from_str(&crate::ssh::output(cmd, 15).await?).map_err(|e| e.to_string())
    }
}
async fn owner(listener: &Listener) -> Result<(u32, String, bool)> {
    #[cfg(unix)]
    {
        for entry in std::fs::read_dir("/proc")
            .map_err(|e| e.to_string())?
            .flatten()
        {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|s| s.parse::<u32>().ok())
            else {
                continue;
            };
            let Ok(fds) = std::fs::read_dir(entry.path().join("fd")) else {
                continue;
            };
            for fd in fds.flatten() {
                if std::fs::read_link(fd.path())
                    .ok()
                    .is_some_and(|p| p.to_string_lossy() == format!("socket:[{}]", listener.inode))
                {
                    let exe = std::fs::read_link(entry.path().join("exe"))
                        .map_err(|_| "权限不足，无法确认监听进程")?;
                    let ssh = exe.file_name().is_some_and(|n| n == "ssh");
                    return Ok((pid, exe.to_string_lossy().into_owned(), ssh));
                }
            }
        }
        Err("监听存在，但权限不足或进程已变化，无法确认来源".into())
    }
    #[cfg(windows)]
    {
        let pid = listener.pid.ok_or("监听未返回 PID")?;
        let mut cmd = platform::command("powershell.exe");
        cmd.env("TETHER_QUERY_PID",pid.to_string()).args(["-NoProfile","-NonInteractive","-Command",r#"$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); $p=Get-CimInstance Win32_Process -Filter ('ProcessId='+$env:TETHER_QUERY_PID); if(-not $p.ExecutablePath){throw '权限不足或系统监听，无法确认进程路径'}; [Console]::Write($p.ExecutablePath)"#]);
        let exe = crate::ssh::output(cmd, 10).await?;
        let ssh = std::path::Path::new(exe.trim())
            .file_name()
            .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("ssh.exe"));
        Ok((pid, exe.trim().into(), ssh))
    }
}
pub async fn occupied(rule: &Rule) -> Result<bool> {
    if rule.direction == Direction::Remote {
        return Ok(false);
    }
    let bind = match rule.bind.as_str() {
        "" | "*" => "0.0.0.0",
        s => s,
    };
    let endpoints = tokio::net::lookup_host((bind, rule.port))
        .await
        .map_err(|e| format!("无法解析监听地址：{e}"))?
        .collect::<Vec<_>>();
    if endpoints.is_empty() {
        return Err("监听地址无可用端点".into());
    }
    for addr in endpoints {
        let socket = if addr.is_ipv4() {
            tokio::net::TcpSocket::new_v4()
        } else {
            tokio::net::TcpSocket::new_v6()
        }
        .map_err(|e| e.to_string())?;
        // OpenSSH uses SO_REUSEADDR on Unix. TIME_WAIT is not an occupied listener.
        #[cfg(unix)]
        socket.set_reuseaddr(true).map_err(|e| e.to_string())?;
        match socket.bind(addr) {
            Ok(()) => drop(socket),
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => return Ok(true),
            Err(e) => return Err(format!("监听端点不可用：{e}")),
        }
    }
    Ok(false)
}
pub async fn test(rt: &Arc<Runtime>, host: &Host) -> Vec<Check> {
    let mut rules = rt
        .store
        .lock()
        .unwrap()
        .config
        .rules
        .iter()
        .filter(|r| r.host_id == host.id)
        .cloned()
        .collect::<Vec<_>>();
    for (_, owned) in rt.owned_rules() {
        for r in owned.into_iter().filter(|r| r.host_id == host.id) {
            if !rules.iter().any(|x| x.id == r.id) {
                rules.push(r);
            }
        }
    }
    let time = now();
    let mut checks = rules
        .into_iter()
        .map(|rule| Check {
            rule_id: rule.id.clone(),
            state: "checking".into(),
            source: None,
            process: None,
            pid: None,
            reason: "正在查询端点占用与可达性".into(),
            checked_at: time,
            reachable: None,
            epoch: rt.detection_epoch.load(std::sync::atomic::Ordering::SeqCst),
            rule,
        })
        .collect::<Vec<_>>();
    publish(rt, &host.id, &checks);
    let listeners = listeners().await;
    for check in &mut checks {
        if check.rule.direction == Direction::Remote {
            check.state = "unverified".into();
            check.reason = "外部远端监听无法在本机确认；托管状态继续实时更新".into();
            continue;
        }
        let rows = match &listeners {
            Ok(rows) => rows,
            Err(e) => {
                check.state = "unavailable".into();
                check.reason = e.clone();
                continue;
            }
        };
        let found = rows
            .iter()
            .filter(|l| l.port == check.rule.port && matches(&l.address, &check.rule.bind))
            .cloned()
            .collect::<Vec<_>>();
        if found.is_empty() {
            match occupied(&check.rule).await {
                Ok(false) => {
                    check.state = "idle".into();
                    check.reason = "端口空闲".into();
                    continue;
                }
                Err(e) => {
                    check.state = "unavailable".into();
                    check.reason = e;
                    continue;
                }
                Ok(true) => {}
            }
        }
        check.reason = "端口占用，正在确认监听来源".into();
        let bind = match check.rule.bind.as_str() {
            "" | "*" | "0.0.0.0" => "127.0.0.1",
            "::" => "::1",
            s => s,
        };
        let probe = tokio::time::timeout(
            Duration::from_millis(600),
            tokio::net::TcpStream::connect((bind, check.rule.port)),
        )
        .await
        .ok()
        .and_then(|r| r.ok());
        check.reachable = Some(probe.is_some());
        publish_one(rt, check);
        let mut owners = vec![];
        let mut reasons = vec![];
        for l in &found {
            match owner(l).await {
                Ok(o) => owners.push(o),
                Err(e) => reasons.push(e),
            }
        }
        if reasons.is_empty() && !owners.is_empty() && owners.iter().all(|(_, _, ssh)| *ssh) {
            check.state = "running".into();
            check.source = Some(rt.environment_label.clone());
            check.pid = Some(owners[0].0);
            check.process = Some(owners[0].1.clone());
            check.reason = "已确认监听属于 SSH；仅观测，无停止或重连权限".into();
        } else {
            check.state = "unavailable".into();
            check.reason = if !reasons.is_empty() {
                reasons.join("；")
            } else {
                if owners.is_empty() {
                    "监听占用，但系统未返回可确认的来源".into()
                } else {
                    "其他程序占用".into()
                }
            };
            if let Some((pid, exe, _)) = owners.first() {
                check.pid = Some(*pid);
                check.process = Some(exe.clone());
                check.source = Some(rt.environment_label.clone());
            }
            #[cfg(windows)]
            if let Some(probe) = probe.as_ref() {
                let relay = owners.is_empty()
                    || owners.iter().all(|(_, exe, _)| {
                        ["wslhost.exe", "wslrelay.exe", "wslservice.exe"]
                            .iter()
                            .any(|n| exe.to_ascii_lowercase().ends_with(n))
                    });
                if relay {
                    if let Ok(peer) = probe.local_addr() {
                        match wsl_owner(&check.rule, peer).await {
                            Ok(Some((source, pid, exe))) => {
                                check.state = "running".into();
                                check.source = Some(source);
                                check.pid = Some(pid);
                                check.process = Some(exe);
                                check.reason =
                                    "已核对当前 TCP 连接与 WSL SSH 监听来源；仅观测".into();
                            }
                            Ok(None) => check.reason.push_str("；运行中的 WSL 未能证明该端点归属"),
                            Err(e) => {
                                check.reason.push('；');
                                check.reason.push_str(&e);
                            }
                        }
                    }
                }
            }
        }
        publish_one(rt, check);
    }
    publish(rt, &host.id, &checks);
    checks
}
fn publish(rt: &Runtime, host: &str, checks: &[Check]) {
    let mut cache = rt.port_checks.lock().unwrap();
    cache.retain(|c| c.rule.host_id != host);
    cache.extend_from_slice(checks);
}
fn publish_one(rt: &Runtime, check: &Check) {
    let mut cache = rt.port_checks.lock().unwrap();
    if let Some(c) = cache.iter_mut().find(|c| c.rule_id == check.rule_id) {
        *c = check.clone();
    }
}

// A matching port alone is never evidence that a Windows endpoint reaches a WSL listener.
// Require the accepted TCP tuple of the probe held open by Windows, in the queried namespace.
pub async fn inspect(v: Value) -> Result<Value> {
    #[cfg(unix)]
    {
        let peer: SocketAddr = v["peer"]
            .as_str()
            .ok_or("缺少探测连接")?
            .parse()
            .map_err(|_| "探测连接无效")?;
        let port = v["port"].as_u64().ok_or("缺少端口")? as u16;
        let sockets = proc_sockets()?;
        let accepted = sockets.iter().find(|(l, s, _, remote)| {
            s == "01"
                && l.port == port
                && proc_endpoint(remote)
                    .is_some_and(|(ip, p)| p == peer.port() && ip == peer.ip().to_string())
        });
        if let Some((accepted, _, _, _)) = accepted {
            if let Ok((pid, exe, true)) = owner(accepted).await {
                return Ok(json!({"pid":pid,"process":exe}));
            }
        }
        Ok(Value::Null)
    }
    #[cfg(not(unix))]
    {
        let _ = v;
        Err("仅用于 Linux 本地端点核对".into())
    }
}
#[cfg(windows)]
async fn wsl_owner(rule: &Rule, peer: SocketAddr) -> Result<Option<(String, u32, String)>> {
    let mut matches = vec![];
    for distro in crate::environments::distributions()
        .await?
        .into_iter()
        .filter(|d| d.running)
    {
        // Recheck running state just before invocation. No intentionally stopped distro is started.
        if !crate::environments::distributions()
            .await?
            .iter()
            .any(|d| d.name == distro.name && d.running)
        {
            continue;
        }
        let exe = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .unwrap()
            .join("tether-linux-x64");
        let bin = platform::wsl_path(&distro.name, &exe).await?;
        let request = json!({"port":rule.port,"peer":peer.to_string()}).to_string();
        let out = platform::wsl(&distro.name, &[&bin, "inspect-port", "--json", &request]).await?;
        let data: Value = serde_json::from_str(&out).map_err(|e| e.to_string())?;
        if let (Some(pid), Some(exe)) = (data["pid"].as_u64(), data["process"].as_str()) {
            matches.push((format!("WSL · {}", distro.name), pid as u32, exe.to_owned()));
        }
    }
    Ok(if matches.len() == 1 {
        matches.pop()
    } else {
        None
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn recent_closed_connections_do_not_block_restart() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let client = tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .unwrap();
        let (accepted, _) = listener.accept().await.unwrap();
        drop(accepted);
        drop(client);
        drop(listener);
        let rule = crate::runtime::inherited_rules(
            "fixture",
            &[format!("dynamicforward [127.0.0.1]:{port}")],
        )
        .unwrap()
        .remove(0);
        assert!(!occupied(&rule).await.unwrap());
    }
    #[test]
    fn endpoints_respect_address_family() {
        assert!(!matches("0.0.0.0", "::1"));
        assert!(!matches("127.0.0.2", "127.0.0.1"));
        assert!(matches("0.0.0.0", "127.0.0.1"));
    }
    #[cfg(unix)]
    #[test]
    fn proc_addresses() {
        assert_eq!(
            proc_endpoint("0100007F:1F90").unwrap(),
            ("127.0.0.1".into(), 8080)
        );
    }
}
