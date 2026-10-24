//! Explicit native expansion, forward-free checked snapshots, and SSH-owned readiness.
use crate::{model::*, store};
use serde::Serialize;
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
    process::Stdio,
    time::Duration,
};
use tokio::process::Command;
pub const MANAGED: &[&str] = &[
    "ForkAfterAuthentication=no",
    "ControlMaster=no",
    "ControlPath=none",
    "ControlPersist=no",
    "PermitLocalCommand=no",
    "RemoteCommand=none",
    "RequestTTY=no",
    "SessionType=none",
    "StdinNull=yes",
    "ExitOnForwardFailure=yes",
    "ClearAllForwardings=no",
    "Tunnel=no",
    "ForwardX11=no",
    "ForwardAgent=no",
];
pub fn key(s: &str) -> String {
    s.split(['=', ' ', '\t'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}
pub fn validate_options(opts: &[String]) -> Result<()> {
    for s in opts {
        let k = key(s);
        if s.contains(['\n', '\r', '\0'])
            || !s.contains('=')
            || k.is_empty()
            || MANAGED.iter().any(|x| key(x) == k)
            || [
                "localforward",
                "remoteforward",
                "dynamicforward",
                "loglevel",
                "logverbose",
                "include",
                "host",
                "match",
                "stdinnull",
                "escapechar",
                "localcommand",
                "knownhostscommand",
            ]
            .contains(&k.as_str())
        {
            return Err(format!(
                "选项 {k} 与托管、转发或交互边界冲突；请使用专用设置"
            ));
        }
    }
    Ok(())
}
pub fn command(program: &str) -> Command {
    crate::platform::command(program)
}
pub async fn output(mut cmd: Command, secs: u64) -> Result<String> {
    cmd.stdin(Stdio::null());
    let out = tokio::time::timeout(Duration::from_secs(secs), cmd.output())
        .await
        .map_err(|_| "本机命令超时（已取消）")?
        .map_err(|e| format!("无法执行本机工具：{e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_owned());
    }
    if out.stdout.len() > 2 * 1024 * 1024 {
        return Err("本机命令输出过大".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
pub fn effective_map(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::<String, Vec<String>>::new();
    for l in text.lines() {
        if let Some((k, v)) = l.split_once(' ') {
            map.entry(k.to_ascii_lowercase())
                .or_default()
                .push(v.into());
        }
    }
    map
}
pub fn nonforward(m: &BTreeMap<String, Vec<String>>) -> BTreeMap<String, Vec<String>> {
    m.iter()
        .filter(|(k, _)| {
            !["localforward", "remoteforward", "dynamicforward", "host"].contains(&k.as_str())
        })
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}
fn native_args(h: &Host) -> Vec<String> {
    let mut args = vec!["-G".into()];
    for o in MANAGED {
        args.extend(["-o".into(), (*o).into()]);
    }
    args.extend(["-o".into(), "LogLevel=DEBUG1".into()]);
    for o in &h.options {
        args.extend(["-o".into(), o.clone()]);
    }
    if !h.options.iter().any(|x| key(x) == "serveraliveinterval") {
        args.extend(["-o".into(), "ServerAliveInterval=15".into()]);
    }
    if !h.options.iter().any(|x| key(x) == "serveralivecountmax") {
        args.extend(["-o".into(), "ServerAliveCountMax=3".into()]);
    }
    if let Some(p) = &h.config_file {
        args.extend(["-F".into(), p.clone()]);
    }
    args.push(h.destination.clone());
    args
}
async fn resolve(h: &Host) -> Result<BTreeMap<String, Vec<String>>> {
    validate_options(&h.options)?;
    let mut cmd = command(&h.ssh);
    cmd.args(native_args(h));
    Ok(effective_map(&output(cmd, h.timeout_secs).await?))
}
fn get(m: &BTreeMap<String, Vec<String>>, key: &str) -> String {
    m.get(key)
        .and_then(|v| v.first())
        .cloned()
        .unwrap_or_default()
}
fn stanza(alias: &str, m: &BTreeMap<String, Vec<String>>) -> String {
    let mut s = format!("Host {alias}\n");
    for option in config_options(m) {
        s.push_str(&format!("  {option}\n"));
    }
    s
}
// Both runtime files and exported -o arguments use SSH configuration quoting.
pub fn config_options(m: &BTreeMap<String, Vec<String>>) -> Vec<String> {
    let mut options = Vec::new();
    for (k, vs) in nonforward(m) {
        for v in vs {
            let v = if [
                "identityfile",
                "certificatefile",
                "identityagent",
                "revokedhostkeys",
                "securitykeyprovider",
                "pkcs11provider",
                "xauthlocation",
            ]
            .contains(&k.as_str())
                && v.contains(' ')
            {
                format!("\"{}\"", v.replace('\\', "/").replace('\"', "\\\""))
            } else {
                v
            };
            options.push(format!("{k} {v}"));
        }
    }
    options
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub host: Host,
    pub effective: BTreeMap<String, Vec<String>>,
    pub inherited_forwards: Vec<String>,
    pub config_text: String,
    pub fingerprint: String,
    #[serde(skip)]
    pub sections: Vec<(String, BTreeMap<String, Vec<String>>)>,
}
pub async fn compile(h: &Host, root: &Path) -> Result<Snapshot> {
    let main = resolve(h).await?;
    for option in ["userknownhostsfile", "globalknownhostsfile"] {
        if let Some(values) = main.get(option) {
            for value in values {
                if value.split_whitespace().any(|p| {
                    p != "none"
                        && !p.starts_with("__PROGRAMDATA__")
                        && !p.starts_with(['/', '~'])
                        && !(p.len() > 2 && p.as_bytes()[1] == b':')
                }) {
                    return Err(format!("{option} 包含相对路径或无法无损区分的空格路径；请改用无空格绝对路径，原文件未修改"));
                }
            }
        }
    }
    let inherited_forwards = main
        .iter()
        .filter(|(k, _)| ["localforward", "remoteforward", "dynamicforward"].contains(&k.as_str()))
        .flat_map(|(k, vs)| vs.iter().map(move |v| format!("{k} {v}")))
        .collect();
    // Snapshot every jump with its own original configuration, never target identity overrides.
    let alias = h.destination.rsplit('@').next().unwrap_or(&h.destination);
    let mut sections = vec![(alias.to_string(), main.clone())];
    let mut visited = HashSet::from([alias.to_string()]);
    let mut i = 0;
    while i < sections.len() {
        let jumps = get(&sections[i].1, "proxyjump");
        for jump in jumps.split(',').filter(|s| !s.is_empty() && *s != "none") {
            let name = jump.rsplit('@').next().unwrap_or(jump);
            let name = if name.starts_with('[') {
                name.split(']')
                    .next()
                    .unwrap_or(name)
                    .trim_start_matches('[')
            } else {
                name.split(':').next().unwrap_or(name)
            };
            if name.contains(':') {
                return Err("首版快照不支持 IPv6 字面量跳板，请使用 SSH 别名".into());
            }
            if !visited.insert(name.to_owned()) {
                return Err("跳板链重复使用别名，无法可靠快照；请为各跳使用独立 SSH 别名".into());
            }
            if sections.len() >= 9 {
                return Err("跳板链过长".into());
            }
            let mut j = h.clone();
            let (destination, port) = if let Some((destination, port)) = jump.rsplit_once(':') {
                (
                    destination.to_string(),
                    Some(port.parse::<u16>().map_err(|_| "跳板端口无效")?),
                )
            } else {
                (jump.to_string(), None)
            };
            j.destination = destination;
            j.options = port.map(|p| vec![format!("Port={p}")]).unwrap_or_default();
            let m = resolve(&j).await?;
            sections.push((name.to_string(), m));
        }
        i += 1;
    }
    let config_text = sections
        .iter()
        .map(|(a, m)| stanza(a, m))
        .collect::<String>();
    let check = root.join(format!("check-{}.conf", id()));
    store::atomic(&check, config_text.as_bytes())?;
    let result = async {
        for (a, m) in &sections {
            let mut cmd = command(&h.ssh);
            cmd.args(["-G", "-F"]).arg(&check).arg(a);
            let reread = effective_map(&output(cmd, h.timeout_secs).await?);
            if nonforward(m) != nonforward(&reread) {
                let differing: Vec<_> = nonforward(m)
                    .keys()
                    .filter(|k| m.get(*k) != reread.get(*k))
                    .cloned()
                    .collect();
                return Err(format!(
                    "SSH 生效配置无法可靠快照：{}；原文件未改动",
                    differing.join(", ")
                ));
            }
        }
        Ok(())
    }
    .await;
    let _ = std::fs::remove_file(check);
    result?;
    let fingerprint = store::revision(
        format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            h.id, h.ssh, config_text, h.reconnect, h.hidden, h.timeout_secs
        )
        .as_bytes(),
    );
    Ok(Snapshot {
        host: h.clone(),
        effective: main,
        inherited_forwards,
        config_text,
        fingerprint,
        sections,
    })
}
pub fn launch_args(snapshot: &Snapshot, file: &Path, rules: &[Rule]) -> Vec<String> {
    let mut args = vec![
        "-N".into(),
        "-T".into(),
        "-F".into(),
        file.to_string_lossy().into_owned(),
    ];
    for r in rules {
        args.push(
            match r.direction {
                Direction::Local => "-L",
                Direction::Remote => "-R",
                Direction::Dynamic => "-D",
            }
            .into(),
        );
        args.push(r.spec());
    }
    args.push(snapshot.host.destination.clone());
    args
}
pub async fn latency(snapshot: &Snapshot) -> Option<f64> {
    if !get(&snapshot.effective, "proxyjump").is_empty()
        && get(&snapshot.effective, "proxyjump") != "none"
        || !get(&snapshot.effective, "proxycommand").is_empty()
            && get(&snapshot.effective, "proxycommand") != "none"
    {
        return None;
    }
    let host = get(&snapshot.effective, "hostname");
    let port = get(&snapshot.effective, "port").parse::<u16>().ok()?;
    let start = std::time::Instant::now();
    tokio::time::timeout(
        Duration::from_secs(3),
        tokio::net::TcpStream::connect((host.as_str(), port)),
    )
    .await
    .ok()?
    .ok()?;
    Some((start.elapsed().as_secs_f64() * 1000.0 * 10.).round() / 10.)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_detaching_and_forward_options() {
        for x in [
            "ForkAfterAuthentication=yes",
            "LocalForward=4 x:5",
            "Include=/tmp/x",
            "LogLevel=QUIET",
            "Host=x\nProxyCommand=bad",
        ] {
            assert!(validate_options(&[x.into()]).is_err());
        }
        assert!(validate_options(&[
            "IdentityFile=C:/a b/key".into(),
            "IdentityFile=C:/second".into(),
            "ProxyJump=jump1,jump2".into()
        ])
        .is_ok());
    }
    #[test]
    fn ipv6_specs() {
        let r = Rule {
            id: "r".into(),
            name: "r".into(),
            host_id: "h".into(),
            direction: Direction::Local,
            bind: "::1".into(),
            port: 80,
            target: "::1".into(),
            target_port: 90,
            notes: "".into(),
            legacy_open_url: serde_json::Value::Null,
        };
        assert_eq!(r.spec(), "[::1]:80:[::1]:90");
    }
}
