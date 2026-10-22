//! Read-only associations with known forwarding intents. Never creates owned groups.
#[cfg(windows)]
use crate::ssh;
use crate::{model::*, runtime::Runtime};
use serde::Serialize;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Match {
    pub rule_id: String,
    pub pid: u32,
    pub verified: bool,
    #[serde(skip)]
    pub rule: Rule,
    #[serde(skip)]
    pub host: Host,
}
#[derive(Default)]
pub struct Cache {
    pub checked: Option<Instant>,
    pub matches: Vec<Match>,
    pub error: Option<String>,
}
pub async fn refresh(rt: &Arc<Runtime>, force: bool) {
    let mut cache = if force {
        rt.external_gate.lock().await
    } else {
        let Ok(cache) = rt.external_gate.try_lock() else {
            return;
        };
        cache
    };
    if !force
        && cache
            .checked
            .is_some_and(|t| t.elapsed() < Duration::from_secs(3))
    {
        return;
    }
    let config = rt.store.lock().unwrap().config.clone();
    let groups = rt.owned_rules();
    let mut rules = config.rules.clone();
    for (_, rs) in &groups {
        for r in rs {
            if !rules.iter().any(|x| x.id == r.id) {
                rules.push(r.clone());
            }
        }
    }
    let mut hosts = config.hosts;
    for (h, _) in groups {
        if !hosts.iter().any(|x| x.id == h.id) {
            hosts.push(h);
        }
    }
    let result = if rules.is_empty() {
        Ok(vec![])
    } else {
        scan(rt, &hosts, &rules).await
    };
    cache.checked = Some(Instant::now());
    match result {
        Ok(matches) => {
            cache.matches = matches;
            cache.error = None;
        }
        Err(e) => {
            cache.matches.clear();
            cache.error = Some(e);
        }
    }
    *rt.external.lock().unwrap() = (cache.matches.clone(), cache.error.clone());
}

// Windows CommandLineToArgvW quoting rules, including escaped quotes and backslashes.
#[cfg_attr(not(windows), allow(dead_code))]
fn arguments(line: &str) -> Option<Vec<String>> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    let mut args = vec![];
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
        }
        if i == chars.len() {
            break;
        }
        let mut arg = String::new();
        let mut quoted = false;
        while i < chars.len() && (quoted || !chars[i].is_whitespace()) {
            let mut slashes = 0;
            while i < chars.len() && chars[i] == '\\' {
                slashes += 1;
                i += 1;
            }
            if i < chars.len() && chars[i] == '"' {
                arg.extend(std::iter::repeat_n('\\', slashes / 2));
                if slashes % 2 == 1 {
                    arg.push('"');
                } else if quoted && chars.get(i + 1) == Some(&'"') {
                    arg.push('"');
                    i += 1;
                } else {
                    quoted = !quoted;
                }
                i += 1;
            } else {
                arg.extend(std::iter::repeat_n('\\', slashes));
                if i < chars.len() && (quoted || !chars[i].is_whitespace()) {
                    arg.push(chars[i]);
                    i += 1;
                }
            }
        }
        if quoted {
            return None;
        }
        args.push(arg);
    }
    Some(args)
}
// Stop at the SSH destination; never pass remote commands or multiplex control actions.
#[cfg_attr(not(windows), allow(dead_code))]
fn config_arguments(args: &[String]) -> Option<Vec<String>> {
    let mut result = vec!["-G".into()];
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if !arg.starts_with('-') {
            result.push(arg.clone());
            return Some(result);
        }
        for (offset, flag) in arg[1..].char_indices() {
            if "BbcDEeFIiJLlmopRSw".contains(flag) {
                result.push(format!("-{flag}"));
                let rest = &arg[offset + 2..];
                if rest.is_empty() {
                    i += 1;
                    result.push(args.get(i)?.clone());
                } else {
                    result.push(rest.into());
                }
                break;
            }
            if !"46AaCfGgKkNnqTtVvXxYy".contains(flag) {
                return None;
            }
            // -G, -V and background mode do not belong in the inspection command.
            if !"fGV".contains(flag) {
                result.push(format!("-{flag}"));
            }
        }
        i += 1;
    }
    None
}
#[cfg_attr(not(windows), allow(dead_code))]
fn address_equal(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
        || a == "localhost" && ["127.0.0.1", "::1"].contains(&b)
        || b == "localhost" && ["127.0.0.1", "::1"].contains(&a)
}
#[cfg_attr(not(windows), allow(dead_code))]
fn intent(a: &Rule, b: &Rule) -> bool {
    a.direction == b.direction
        && a.port == b.port
        && address_equal(&a.bind, &b.bind)
        && (a.direction == Direction::Dynamic
            || a.target.eq_ignore_ascii_case(&b.target) && a.target_port == b.target_port)
}
// Only explicit forwarding arguments are evidence of the old process's intent.
// Do not run ssh -G while polling: Match exec can execute arbitrary local commands.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Default)]
struct Context {
    destination: String,
    user: Option<String>,
    port: Option<u16>,
    config_file: Option<String>,
    forwards: Vec<Rule>,
}
impl Context {
    #[cfg_attr(not(windows), allow(dead_code))]
    fn same_host(&self, other: &Self) -> bool {
        self.destination.eq_ignore_ascii_case(&other.destination)
            && self.user == other.user
            && self.port == other.port
            && self
                .config_file
                .as_deref()
                .map(|s| s.replace('\\', "/").to_lowercase())
                == other
                    .config_file
                    .as_deref()
                    .map(|s| s.replace('\\', "/").to_lowercase())
    }
}
#[cfg_attr(not(windows), allow(dead_code))]
fn parts(s: &str) -> Vec<String> {
    let mut result = vec![];
    let mut part = String::new();
    let mut bracket = false;
    for c in s.chars() {
        match c {
            '[' => bracket = true,
            ']' => bracket = false,
            ':' if !bracket => {
                result.push(part);
                part = String::new();
            }
            _ => part.push(c),
        }
    }
    result.push(part);
    result
}
#[cfg_attr(not(windows), allow(dead_code))]
fn forward(kind: &str, spec: &str) -> Option<Rule> {
    let columns = if spec.contains(char::is_whitespace) {
        let mut p = spec.split_whitespace();
        let mut c = parts(p.next()?);
        if let Some(target) = p.next() {
            c.extend(parts(target));
        }
        c
    } else {
        parts(spec)
    };
    let (bind, port, target, target_port) = match (kind, columns.as_slice()) {
        ("dynamicforward", [p]) => (
            "localhost".to_string(),
            p.clone(),
            String::new(),
            "0".into(),
        ),
        ("dynamicforward", [b, p]) => (b.clone(), p.clone(), String::new(), "0".into()),
        (_, [p, t, tp]) => ("localhost".into(), p.clone(), t.clone(), tp.clone()),
        (_, [b, p, t, tp]) => (b.clone(), p.clone(), t.clone(), tp.clone()),
        _ => return None,
    };
    Some(Rule {
        id: "external".into(),
        name: "external".into(),
        host_id: "external".into(),
        direction: match kind {
            "localforward" => Direction::Local,
            "remoteforward" => Direction::Remote,
            "dynamicforward" => Direction::Dynamic,
            _ => return None,
        },
        bind,
        port: port.parse().ok()?,
        target,
        target_port: target_port.parse().ok()?,
        notes: String::new(),
        legacy_open_url: serde_json::Value::Null,
    })
}
#[cfg_attr(not(windows), allow(dead_code))]
fn context(args: &[String]) -> Option<Context> {
    let mut c = Context::default();
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        if !arg.starts_with('-') {
            let (user, host) = arg
                .rsplit_once('@')
                .map(|(u, h)| (Some(u.to_string()), h))
                .unwrap_or((None, arg));
            if user.is_some() {
                c.user = user;
            }
            c.destination = host.into();
            return Some(c);
        }
        if ["-F", "-p", "-l", "-L", "-R", "-D", "-o"].contains(&arg.as_str()) {
            i += 1;
            let v = args.get(i)?;
            match arg.as_str() {
                "-F" => c.config_file = Some(v.clone()),
                "-p" => c.port = Some(v.parse().ok()?),
                "-l" => c.user = Some(v.clone()),
                "-L" => c.forwards.push(forward("localforward", v)?),
                "-R" => c.forwards.push(forward("remoteforward", v)?),
                "-D" => c.forwards.push(forward("dynamicforward", v)?),
                "-o" => {
                    let (key, v) = v.split_once('=').or_else(|| v.split_once(' '))?;
                    match key.to_lowercase().as_str() {
                        "port" => c.port = Some(v.parse().ok()?),
                        "user" => c.user = Some(v.into()),
                        "localforward" | "remoteforward" | "dynamicforward" => {
                            c.forwards.push(forward(&key.to_lowercase(), v)?)
                        }
                        "hostname" | "proxyjump" | "proxycommand" | "clearallforwardings" => {
                            return None
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        } else if arg.len() == 2 && "BbcEeIiJLmSw".contains(arg.chars().nth(1)?) {
            i += 1;
            args.get(i)?;
            if arg == "-J" {
                return None;
            }
        }
        i += 1;
    }
    None
}
#[cfg_attr(not(windows), allow(dead_code))]
fn host_context(h: &Host) -> Option<Context> {
    let mut args = vec!["-G".into()];
    if let Some(f) = &h.config_file {
        args.extend(["-F".into(), f.clone()]);
    }
    for option in &h.options {
        args.extend(["-o".into(), option.clone()]);
    }
    args.push(h.destination.clone());
    context(&args)
}
#[cfg(windows)]
async fn scan(rt: &Runtime, hosts: &[Host], rules: &[Rule]) -> Result<Vec<Match>> {
    use serde_json::Value;
    let mut cmd = ssh::command("powershell.exe");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", r#"$ErrorActionPreference='Stop'; [Console]::OutputEncoding=[System.Text.UTF8Encoding]::new(); $p=@(Get-CimInstance Win32_Process -Filter "Name='ssh.exe'" | Select-Object ProcessId,CommandLine); $ids=@($p.ProcessId); $tcp=@(); if($ids.Count){$tcp=@(Get-NetTCPConnection -ErrorAction Stop | Where-Object {$ids -contains $_.OwningProcess} | Select-Object OwningProcess,LocalAddress,LocalPort,RemoteAddress,RemotePort,@{n='State';e={$_.State.ToString()}})}; @{processes=$p;tcp=$tcp} | ConvertTo-Json -Depth 4 -Compress"#]);
    let out = ssh::output(cmd, 10).await?;
    let data: Value = serde_json::from_str(out.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("无法读取终端 SSH 状态：{e}"))?;
    let owned = rt.owned_pids();
    let mut matches = vec![];
    for p in data["processes"].as_array().into_iter().flatten() {
        let Some(pid) = p["ProcessId"].as_u64().map(|n| n as u32) else {
            continue;
        };
        if owned.contains(&pid) {
            continue;
        }
        let Some(args) = p["CommandLine"]
            .as_str()
            .and_then(arguments)
            .and_then(|a| config_arguments(&a))
        else {
            continue;
        };
        let Some(context) = context(&args) else {
            continue;
        };
        let sockets = data["tcp"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|s| s["OwningProcess"].as_u64() == Some(pid as u64))
            .collect::<Vec<_>>();
        let listen_ports = sockets
            .iter()
            .filter(|s| s["State"] == "Listen")
            .filter_map(|s| s["LocalPort"].as_u64())
            .collect::<Vec<_>>();
        // Require an established SSH socket, not merely a listener or a waiting password prompt.
        if !sockets.iter().any(|s| {
            s["State"] == "Established"
                && !listen_ports.contains(&s["LocalPort"].as_u64().unwrap_or(0))
                && context
                    .port
                    .is_none_or(|p| s["RemotePort"].as_u64() == Some(p as u64))
        }) {
            continue;
        }
        for h in hosts {
            let Some(expected) = host_context(h) else {
                continue;
            };
            if !context.same_host(&expected) {
                continue;
            }
            for r in rules.iter().filter(|r| r.host_id == h.id) {
                if !context.forwards.iter().any(|f| intent(r, f)) {
                    continue;
                }
                let verified = r.direction != Direction::Remote
                    && sockets.iter().any(|s| {
                        s["State"] == "Listen"
                            && s["LocalPort"].as_u64() == Some(r.port as u64)
                            && s["LocalAddress"]
                                .as_str()
                                .is_some_and(|a| address_equal(a, &r.bind))
                    });
                if r.direction != Direction::Remote && !verified {
                    continue;
                }
                matches.push(Match {
                    rule_id: r.id.clone(),
                    pid,
                    verified,
                    rule: r.clone(),
                    host: h.clone(),
                });
            }
        }
    }
    Ok(matches)
}
#[cfg(not(windows))]
async fn scan(_rt: &Runtime, _hosts: &[Host], _rules: &[Rule]) -> Result<Vec<Match>> {
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quoted_windows_arguments_and_safe_config_inspection() {
        let args = arguments(r#""C:\Program Files\OpenSSH\ssh.exe" -NT -F "C:\SSH Files\config" -L127.0.0.1:18080:127.0.0.1:8080 fixture echo secret"#).unwrap();
        assert_eq!(args[0], r"C:\Program Files\OpenSSH\ssh.exe");
        assert_eq!(
            config_arguments(&args).unwrap(),
            vec![
                "-G",
                "-N",
                "-T",
                "-F",
                r"C:\SSH Files\config",
                "-L",
                "127.0.0.1:18080:127.0.0.1:8080",
                "fixture"
            ]
        );
        assert!(config_arguments(&arguments("ssh -O exit fixture").unwrap()).is_none());
        assert!(config_arguments(&arguments("ssh -W host:22 fixture").unwrap()).is_none());
        assert!(arguments("ssh -F \"unterminated").is_none());
        assert_eq!(
            arguments(r#"ssh -o "IdentityFile=C:\SSH\a\"b" fixture"#).unwrap()[2],
            "IdentityFile=C:\\SSH\\a\"b"
        );
    }
    #[test]
    fn explicit_ports_users_and_ipv6_forwards_match_without_native_config_execution() {
        let c = context(
            &config_arguments(
                &arguments("ssh -NT -p2222 -L[::1]:8080:[::1]:80 -D1080 admin@server").unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(c.destination, "server");
        assert_eq!(c.port, Some(2222));
        assert_eq!(c.user.as_deref(), Some("admin"));
        assert_eq!(c.forwards[0].target, "::1");
        assert_eq!(c.forwards[1].direction, Direction::Dynamic);
        let mut expected = Context {
            destination: "server".into(),
            user: Some("admin".into()),
            port: Some(2222),
            ..Context::default()
        };
        assert!(c.same_host(&expected));
        expected.port = Some(22);
        assert!(!c.same_host(&expected));
        assert!(forward("localforward", "/tmp/socket:/tmp/target").is_none());
        assert!(context(
            &config_arguments(&arguments("ssh -J jump -L 8080:host:80 server").unwrap()).unwrap()
        )
        .is_none());
    }
    #[test]
    fn matches_full_forward_including_target_and_direction() {
        let r = crate::runtime::inherited_rules(
            "h",
            &["localforward [127.0.0.1]:18080 [127.0.0.1]:8080".into()],
        )
        .unwrap()
        .remove(0);
        let mut other = r.clone();
        other.bind = "localhost".into();
        assert!(intent(&r, &other));
        other.target_port += 1;
        assert!(!intent(&r, &other));
        other = r.clone();
        other.direction = Direction::Remote;
        assert!(!intent(&r, &other));
        other = r.clone();
        other.bind = "0.0.0.0".into();
        assert!(!intent(&r, &other));
    }
}
