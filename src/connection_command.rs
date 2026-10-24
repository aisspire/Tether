//! Single-forward, file-independent commands. Generation runs only ssh -G, never connects.
use crate::{
    model::*,
    ssh::{self, Snapshot},
};
use serde::Serialize;
use std::collections::BTreeMap;

type Options = BTreeMap<String, Vec<String>>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionCommand {
    program: String,
    args: Vec<String>,
    pub command: String,
    shell: &'static str,
}

fn first<'a>(options: &'a Options, key: &str) -> &'a str {
    options
        .get(key)
        .and_then(|v| v.first())
        .map(String::as_str)
        .unwrap_or("")
}
fn set(options: &mut Options, key: &str, value: impl Into<String>) {
    options.insert(key.into(), vec![value.into()]);
}
fn uses_jump_token(value: &str) -> bool {
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' && matches!(chars.next(), Some('j' | 'C')) {
            return true;
        }
    }
    false
}

// POSIX shell quoting and Windows native argv quoting are separate layers.
fn posix(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&b))
    {
        value.into()
    } else {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
}
fn windows(value: &str) -> String {
    let mut quoted = String::from("\"");
    let mut slashes = 0;
    for ch in value.chars() {
        if ch == '\\' {
            slashes += 1;
        } else {
            quoted.push_str(&"\\".repeat(if ch == '"' { slashes * 2 + 1 } else { slashes }));
            quoted.push(ch);
            slashes = 0;
        }
    }
    quoted.push_str(&"\\".repeat(slashes * 2));
    quoted.push('"');
    quoted
}
fn powershell(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
fn native_line(program: &str, args: &[String], win: bool) -> String {
    std::iter::once(program)
        .chain(args.iter().map(String::as_str))
        .map(|s| if win { windows(s) } else { posix(s) })
        .collect::<Vec<_>>()
        .join(" ")
}
fn render(program: &str, args: &[String], win: bool) -> String {
    if win {
        // Pass one native argument string, preserving embedded quotes on both
        // Windows PowerShell 5.1 and PowerShell 7, without --% environment expansion.
        let arguments = args
            .iter()
            .map(|s| windows(s))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "Start-Process -FilePath {} -ArgumentList {} -NoNewWindow -Wait",
            powershell(program),
            powershell(&arguments)
        )
    } else {
        native_line(program, args, false)
    }
}

async fn inspect(program: &str, args: &[String], secs: u64) -> Result<Options> {
    let mut cmd = ssh::command(program);
    cmd.arg("-G").args(args);
    Ok(ssh::effective_map(&ssh::output(cmd, secs).await?))
}

async fn host_args(
    snapshot: &Snapshot,
    destination: &str,
    source: &Options,
    rule: Option<&Rule>,
    relay: Option<String>,
    preceding_jumps: Option<String>,
    socket: Option<&str>,
    depth: usize,
) -> Result<Vec<String>> {
    if depth > 8 {
        return Err("跳板链过长，无法生成完整链接命令".into());
    }
    let mut expected = ssh::nonforward(source);
    set(&mut expected, "loglevel", "INFO");
    if relay.is_some() {
        // -W relays must read the parent SSH stream, and must not inherit forwards.
        set(&mut expected, "stdinnull", "no");
        set(&mut expected, "clearallforwardings", "yes");
    }
    if let Some(socket) = socket {
        if matches!(
            first(&expected, "identityagent"),
            "" | "SSH_AUTH_SOCK" | "$SSH_AUTH_SOCK"
        ) {
            set(&mut expected, "identityagent", socket);
        }
    }
    if let Some(jumps) = preceding_jumps {
        expected.remove("proxycommand");
        set(&mut expected, "proxyjump", jumps);
    }
    let jumps = first(&expected, "proxyjump").to_owned();
    if !jumps.is_empty() && jumps != "none" {
        if expected
            .iter()
            .any(|(key, values)| key != "proxyjump" && values.iter().any(|v| uses_jump_token(v)))
        {
            return Err(
                "连接参数使用依赖 ProxyJump 的 %j 或 %C token，无法无损导出完整链接命令".into(),
            );
        }
        let (prefix, last) = jumps
            .rsplit_once(',')
            .map_or((None, jumps.as_str()), |(p, j)| (Some(p.to_owned()), j));
        // compile() already validates the jump syntax and resolves each alias.
        let jump_destination = last.rsplit_once(':').map_or(last, |(d, _)| d);
        let alias = jump_destination
            .rsplit('@')
            .next()
            .unwrap_or(jump_destination);
        let jump = snapshot
            .sections
            .iter()
            .find(|(name, _)| name == alias)
            .ok_or("跳板快照缺失，无法生成完整链接命令")?;
        let endpoint = format!(
            "[{}]:{}",
            first(&expected, "hostname"),
            first(&expected, "port")
        );
        let child = Box::pin(host_args(
            snapshot,
            jump_destination,
            &jump.1,
            None,
            Some(endpoint),
            prefix,
            socket,
            depth + 1,
        ))
        .await?;
        // ProxyCommand expands percent tokens once before starting its child.
        let proxy = native_line(&snapshot.host.ssh, &child, cfg!(windows)).replace('%', "%%");
        expected.remove("proxyjump");
        set(&mut expected, "proxycommand", proxy);
    }

    let mut args = vec!["-F".into(), "none".into(), "-N".into(), "-T".into()];
    if let Some(endpoint) = relay {
        args.extend(["-W".into(), endpoint]);
    }
    // Compare against the same client's defaults so common commands stay short.
    // -F none prevents config forwards, Include and Match from leaking back in.
    let mut baseline_args = args.clone();
    baseline_args.push(destination.into());
    let baseline = inspect(
        &snapshot.host.ssh,
        &baseline_args,
        snapshot.host.timeout_secs,
    )
    .await?;
    let delta = expected
        .iter()
        .filter(|(k, v)| baseline.get(*k) != Some(*v))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for option in ssh::config_options(&delta) {
        args.extend(["-o".into(), option]);
    }
    if let Some(rule) = rule {
        args.extend([
            match rule.direction {
                Direction::Local => "-L",
                Direction::Remote => "-R",
                Direction::Dynamic => "-D",
            }
            .into(),
            rule.spec(),
        ]);
    }
    args.push(destination.into());
    if cfg!(windows)
        && native_line(&snapshot.host.ssh, &args, true)
            .encode_utf16()
            .count()
            > 30_000
    {
        return Err("完整链接命令超过 Windows 命令长度限制，请简化连接参数或跳板链".into());
    }
    let actual = inspect(&snapshot.host.ssh, &args, snapshot.host.timeout_secs).await?;
    if ssh::nonforward(&actual) != expected {
        let keys: Vec<_> = expected
            .keys()
            .chain(actual.keys())
            .filter(|k| {
                !["host", "localforward", "remoteforward", "dynamicforward"].contains(&k.as_str())
                    && expected.get(*k) != actual.get(*k)
            })
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        return Err(format!(
            "SSH 参数无法无损导出（{}），未复制命令",
            keys.join("、")
        ));
    }
    Ok(args)
}

pub async fn generate(
    snapshot: &Snapshot,
    rule: &Rule,
    socket: Option<&str>,
) -> Result<ConnectionCommand> {
    rule.validate()?;
    let args = host_args(
        snapshot,
        &snapshot.host.destination,
        &snapshot.effective,
        Some(rule),
        None,
        None,
        socket,
        0,
    )
    .await?;
    let program = snapshot.host.ssh.clone();
    let command = render(&program, &args, cfg!(windows));
    if command.contains(['\n', '\r', '\0']) {
        return Err("连接参数包含换行或空字符，无法复制为单行命令".into());
    }
    Ok(ConnectionCommand {
        program,
        args,
        command,
        shell: if cfg!(windows) { "PowerShell" } else { "POSIX" },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quoting_keeps_literals_and_windows_native_arguments() {
        assert_eq!(posix("a'b $(touch x)"), "'a'\"'\"'b $(touch x)'");
        assert_eq!(windows("a\\\"b\\"), "\"a\\\\\\\"b\\\\\"");
        assert_eq!(powershell("a'b $env:X"), "'a''b $env:X'");
        let command = render(
            "C:/Program Files/ssh.exe",
            &["IdentityFile=C:/a\"b/".into()],
            true,
        );
        assert!(command.starts_with("Start-Process -FilePath 'C:/Program Files/ssh.exe'"));
        assert!(command.ends_with(" -NoNewWindow -Wait"));
        assert!(uses_jump_token("key-%j-%h"));
        assert!(uses_jump_token("key-%C"));
        assert!(!uses_jump_token("key-%%j-%%C"));
    }
}
