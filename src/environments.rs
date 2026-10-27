use crate::{bridge::Bridge, model::*, platform, runtime::Runtime, store};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    process::Stdio,
    sync::{Arc, Mutex},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    pub id: String,
    pub label: String,
    pub distro: String,
    pub user: String,
    pub uid: String,
    #[serde(default)]
    pub home: String,
}
#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Registry {
    #[serde(default)]
    environments: Vec<Environment>,
    selected: Option<String>,
}
#[derive(Serialize)]
pub struct Distribution {
    pub name: String,
    pub running: bool,
    pub version: u32,
}
pub async fn distributions() -> Result<Vec<Distribution>> {
    if !cfg!(windows) {
        return Ok(vec![]);
    }
    async fn list(args: &[&str]) -> Result<String> {
        let mut cmd = platform::command("wsl.exe");
        cmd.args(args);
        let out = tokio::time::timeout(std::time::Duration::from_secs(10), cmd.output())
            .await
            .map_err(|_| "WSL 枚举超时")?
            .map_err(|e| format!("WSL 不可用：{e}"))?;
        if !out.status.success() {
            return Err(format!("WSL 不可用：{}", platform::decode(&out.stdout)));
        }
        Ok(platform::decode(&out.stdout))
    }
    let names = list(&["--list", "--quiet"]).await?;
    let running = list(&["--list", "--running", "--quiet"]).await?;
    let verbose = list(&["--list", "--verbose"]).await?;
    Ok(parse_distributions(&names, &running, &verbose))
}
fn parse_distributions(names: &str, running: &str, verbose: &str) -> Vec<Distribution> {
    names
        .lines()
        .map(str::trim)
        .filter(|n| !n.is_empty() && !n.to_ascii_lowercase().starts_with("docker-desktop"))
        .map(|name| {
            let version = verbose
                .lines()
                .find_map(|line| {
                    let line = line.trim().trim_start_matches('*').trim();
                    line.strip_prefix(name)
                        .filter(|s| s.starts_with(char::is_whitespace))
                        .and_then(|s| s.split_whitespace().last()?.parse().ok())
                })
                .unwrap_or(0);
            Distribution {
                name: name.into(),
                running: running.lines().any(|s| s.trim() == name),
                version,
            }
        })
        .filter(|d| d.version == 2)
        .collect()
}
pub struct Hub {
    pub local: Arc<Runtime>,
    data: PathBuf,
    registry: Mutex<Registry>,
    sessions: tokio::sync::Mutex<HashMap<String, Arc<Bridge>>>,
    selection: tokio::sync::Mutex<()>,
    terminal_gate: tokio::sync::Mutex<()>,
}
impl Hub {
    pub fn new(local: Arc<Runtime>, data: PathBuf) -> Result<Arc<Self>> {
        let path = data.join("environments.json");
        let registry: Registry = if path.exists() {
            serde_json::from_slice(&store::read(&path)?)
                .map_err(|e| format!("环境登记无法读取：{e}"))?
        } else {
            Registry::default()
        };
        let mut ids = std::collections::HashSet::new();
        for env in &registry.environments {
            if env.id != environment_id(&env.distro, &env.uid, &env.user)
                || !ids.insert(&env.id)
                || env.distro.is_empty()
                || env.user.is_empty()
                || env.uid.parse::<u32>().is_err()
            {
                return Err("环境登记身份无效，请修复 environments.json；原文件已保留".into());
            }
        }
        Ok(Arc::new(Self {
            local,
            data,
            registry: Mutex::new(registry),
            sessions: tokio::sync::Mutex::default(),
            selection: tokio::sync::Mutex::new(()),
            terminal_gate: tokio::sync::Mutex::new(()),
        }))
    }
    fn persist(&self, r: &Registry) -> Result<()> {
        store::atomic(
            &self.data.join("environments.json"),
            &serde_json::to_vec_pretty(r).unwrap(),
        )
    }
    pub async fn exit(&self) {
        self.local.begin_exit();
        let sessions = self
            .sessions
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        let mut tasks = tokio::task::JoinSet::new();
        for session in sessions {
            tasks.spawn(async move {
                session.close().await;
            });
        }
        self.local.exit().await;
        while tasks.join_next().await.is_some() {}
    }
    async fn identity(&self, distro: &str) -> Result<Environment> {
        if !distributions().await?.iter().any(|d| d.name == distro) {
            return Err("没有可用的 WSL 2 用户发行版，已取消切换".into());
        }
        let uid = platform::wsl(distro, &["id", "-u"]).await?;
        let user = platform::wsl(distro, &["id", "-un"]).await?;
        if platform::wsl(distro, &["uname", "-m"]).await? != "x86_64" {
            return Err("本包仅支持 WSL 2 x64".into());
        }
        let home = platform::wsl(distro, &["sh", "-c", "printf '%s' \"$HOME\""]).await?;
        let id = environment_id(distro, &uid, &user);
        Ok(Environment {
            id,
            label: format!("WSL · {distro} · {user}"),
            distro: distro.into(),
            user,
            uid,
            home,
        })
    }
    async fn connect(&self, env: &Environment) -> Result<Arc<Bridge>> {
        let mut sessions = self.sessions.lock().await;
        if let Some(s) = sessions.get(&env.id).filter(|s| s.alive()) {
            return Ok(s.clone());
        }
        if *self.local.shutdown.borrow() {
            return Err("后台正在退出".into());
        }
        let exe = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .unwrap()
            .join("tether-linux-x64");
        if !exe.is_file() {
            return Err("随包 Linux x64 后台缺失，请重新构建完整目录包".into());
        }
        let data = self.data.join("environments").join(&env.id);
        store::private_dir(&data)?;
        let bin = platform::wsl_path(&env.distro, &exe).await?;
        let data = platform::wsl_path(&env.distro, &data).await?;
        let mut cmd = platform::command("wsl.exe");
        cmd.args([
            "--distribution",
            &env.distro,
            "--exec",
            &bin,
            "serve",
            "--bridge-stdio",
            "--data-dir",
            &data,
            "--environment",
            &env.id,
            "--environment-label",
            &env.label,
            "--expected-uid",
            &env.uid,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
        let session = Bridge::connect(cmd.spawn().map_err(|e| e.to_string())?, &env.id).await?;
        sessions.insert(env.id.clone(), session.clone());
        Ok(session)
    }
    async fn session(&self, id: &str) -> Result<Arc<Bridge>> {
        // Existing sessions retain their user even if the distribution's default user changes.
        if let Some(s) = self
            .sessions
            .lock()
            .await
            .get(id)
            .filter(|s| s.alive())
            .cloned()
        {
            return Ok(s);
        }
        let env = self
            .registry
            .lock()
            .unwrap()
            .environments
            .iter()
            .find(|e| e.id == id)
            .cloned()
            .ok_or("未知环境，请重新选择")?;
        let actual = self.identity(&env.distro).await?;
        if actual.id != env.id {
            return Err("WSL 默认用户已改变，请重新选择该发行版；旧用户配置已保留".into());
        }
        self.connect(&env).await
    }
    pub async fn action(self: &Arc<Self>, v: Value) -> Result<Value> {
        if v["action"] == "shutdown" || v["action"] == "status" {
            return self.dispatch(v).await;
        }
        let mut shutdown = self.local.shutdown.subscribe();
        if *shutdown.borrow() {
            return Err("后台正在退出".into());
        }
        tokio::select! {
            result=self.dispatch(v)=>result,
            _=shutdown.changed()=>Err("操作已因后台退出取消".into()),
        }
    }
    async fn dispatch(self: &Arc<Self>, mut v: Value) -> Result<Value> {
        let action = v["action"].as_str().unwrap_or("").to_owned();
        if *self.local.shutdown.borrow() && action != "status" {
            return Err("后台正在退出".into());
        }
        if action == "shutdown" {
            self.exit().await;
            return Ok(json!({"stopped":true,"instance":self.local.instance}));
        }
        if action == "environments.list" {
            let ds = distributions().await;
            let r = self.registry.lock().unwrap();
            return Ok(
                json!({"local":{"id":self.local.environment,"label":self.local.environment_label},"distributions":ds.as_ref().ok(),"error":ds.as_ref().err(),"environments":r.environments,"selected":r.selected}),
            );
        }
        if action == "environments.select" {
            let _selection = self.selection.lock().await;
            let selected = if let Some(distro) = v["distro"].as_str() {
                let env = self.identity(distro).await?;
                let session = self.connect(&env).await?;
                let state = session.call(json!({"action":"activate"})).await?;
                let mut r = self.registry.lock().unwrap();
                if !r.environments.iter().any(|e| e.id == env.id) {
                    r.environments.push(env.clone());
                }
                r.selected = Some(env.id.clone());
                self.persist(&r)?;
                json!({"environment":env,"state":state})
            } else {
                self.local.invalidate_checks();
                self.local.activate();
                let mut r = self.registry.lock().unwrap();
                r.selected = Some(self.local.environment.clone());
                self.persist(&r)?;
                json!({"environment":{"id":self.local.environment,"label":self.local.environment_label},"state":self.local.status()})
            };
            return Ok(selected);
        }
        let env = v["environment"]
            .as_str()
            .unwrap_or(&self.local.environment)
            .to_owned();
        if action == "hostTerminal" {
            let _gate = self.terminal_gate.lock().await;
            v["action"] = json!("hostCommand");
            v["checkAvailable"] = json!(true);
            v.as_object_mut().unwrap().remove("environment");
            let (command, environment) = if env == self.local.environment {
                (self.local.action(v).await?, None)
            } else {
                let session = self.session(&env).await?;
                let environment = self
                    .registry
                    .lock()
                    .unwrap()
                    .environments
                    .iter()
                    .find(|e| e.id == env)
                    .cloned()
                    .ok_or("未知环境，请重新选择")?;
                (session.call(v).await?, Some(environment))
            };
            if *self.local.shutdown.borrow() {
                return Err("后台正在退出，已取消终端启动".into());
            }
            let terminal_pid = crate::terminal::open(
                serde_json::from_value(command).map_err(|e| e.to_string())?,
                environment.as_ref(),
            )?;
            return Ok(json!({"opened":true,"environment":env,"terminalPid":terminal_pid}));
        }
        if action == "files.choose" {
            if env == self.local.environment {
                if !cfg!(windows) {
                    return self
                        .local
                        .action(json!({"action":"files.browse","path":v["path"]}))
                        .await;
                }
                let p = platform::choose_file("").await?;
                return match p {
                    Some(p) => {
                        self.local
                            .action(json!({"action":"files.open","path":p}))
                            .await
                    }
                    None => Ok(json!({"cancelled":true})),
                };
            }
            return self
                .session(&env)
                .await?
                .call(json!({"action":"files.browse","path":v["path"]}))
                .await;
        }
        v.as_object_mut()
            .ok_or("请求须为对象")?
            .remove("environment");
        if env == self.local.environment {
            if action == "ensure" {
                self.local.activate();
            }
            self.local.action(v).await
        } else {
            self.session(&env).await?.call(v).await
        }
    }
}
fn environment_id(distro: &str, uid: &str, user: &str) -> String {
    format!(
        "wsl-{}",
        &store::revision(format!("{distro}\0{uid}\0{user}").as_bytes())[..24]
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_users_have_stable_separate_configuration_ids() {
        let id = environment_id("Ubuntu", "1000", "user");
        assert_eq!(id, environment_id("Ubuntu", "1000", "user"));
        assert_ne!(id, environment_id("Ubuntu", "1001", "other"));
        assert_ne!(id, environment_id("Debian", "1000", "user"));
    }
    #[test]
    fn multiple_distributions_and_localized_states() {
        let ds = parse_distributions(
            "Ubuntu\nDebian\nOld\ndocker-desktop",
            "Debian",
            "* Ubuntu    已停止 2\n Debian 正在运行 2\n Old 已停止 1\n docker-desktop 正在运行 2",
        );
        assert_eq!(ds.len(), 2);
        assert_eq!(ds[0].name, "Ubuntu");
        assert!(!ds[0].running);
        assert!(ds[1].running);
    }
}
