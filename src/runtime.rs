use crate::{
    model::*,
    owned::Owned,
    ssh::{self, Snapshot},
    store::{self, Store},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    sync::{oneshot, watch},
};
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub host_id: String,
    pub rules: Vec<Rule>,
    pub consumers: Vec<String>,
    pub state: String,
    pub error: Option<String>,
    pub pid: Option<u32>,
    pub attempts: u32,
    pub snapshot: Snapshot,
    pub args: Vec<String>,
    pub logs: Vec<String>,
    pub desired: bool,
}
struct Entry {
    view: Group,
    cancel: watch::Sender<bool>,
    status: watch::Sender<String>,
}
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Prompt {
    pub id: String,
    pub group_id: String,
    pub host: String,
    pub text: String,
    pub secret: bool,
}
struct Pending {
    view: Prompt,
    answer: oneshot::Sender<String>,
}
#[derive(Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ensure {
    #[serde(default)]
    pub rule_ids: Vec<String>,
    #[serde(default)]
    pub temporary: Vec<Rule>,
    #[serde(default)]
    pub overrides: Vec<String>,
    #[serde(default)]
    pub consumer: String,
    pub plan_id: Option<String>,
}
pub struct Runtime {
    pub root: PathBuf,
    pub store: Mutex<Store>,
    groups: Mutex<HashMap<String, Entry>>,
    gate: tokio::sync::Mutex<()>,
    epoch: AtomicU64,
    prompts: Mutex<HashMap<String, Pending>>,
    pub origin: String,
    pub token: String,
    pub instance: String,
    pub auth_token: String,
    pub auth_origin: String,
    latency: Mutex<HashMap<String, Option<f64>>>,
    pub shutdown: watch::Sender<bool>,
    pub tool_cancel: watch::Sender<u64>,
}
impl Runtime {
    pub fn new(root: PathBuf, origin: String, token: String, auth_origin: String) -> Arc<Self> {
        let (shutdown, _) = watch::channel(false);
        Arc::new(Self {
            store: Mutex::new(Store::open(&root)),
            root,
            groups: Mutex::default(),
            gate: tokio::sync::Mutex::new(()),
            epoch: AtomicU64::new(0),
            prompts: Mutex::default(),
            origin,
            token,
            instance: id(),
            auth_token: format!("{}{}", id(), id()),
            auth_origin,
            latency: Mutex::default(),
            shutdown,
            tool_cancel: watch::channel(0).0,
        })
    }
    pub fn status(&self) -> Value {
        let s = self.store.lock().unwrap();
        let groups = self
            .groups
            .lock()
            .unwrap()
            .values()
            .map(|e| e.view.clone())
            .collect::<Vec<_>>();
        let prompts = self
            .prompts
            .lock()
            .unwrap()
            .values()
            .map(|p| p.view.clone())
            .collect::<Vec<_>>();
        json!({"protocol":1,"instance":self.instance,"config":s.config,"revision":s.revision,"configError":s.error,"configPath":s.path,"groups":groups,"prompts":prompts,"latency":*self.latency.lock().unwrap()})
    }
    pub async fn action(self: &Arc<Self>, value: Value) -> Result<Value> {
        if *self.shutdown.borrow() && value["action"] != "status" {
            return Err("后台正在退出，已拒绝新操作".into());
        }
        match value.get("action").and_then(Value::as_str).unwrap_or("") {
            "status" => Ok(self.status()),
            "reload" => {
                *self.store.lock().unwrap() = Store::open(&self.root);
                Ok(self.status())
            }
            "save" => {
                let c =
                    serde_json::from_value(value["config"].clone()).map_err(|e| e.to_string())?;
                self.store
                    .lock()
                    .unwrap()
                    .save(c, value["revision"].as_str().ok_or("缺少配置版本")?)?;
                Ok(self.status())
            }
            "ensure" => {
                self.ensure(
                    serde_json::from_value(value.get("request").cloned().unwrap_or(json!({})))
                        .map_err(|e| e.to_string())?,
                )
                .await
            }
            "stop" => {
                let scope = value["scope"].as_str().ok_or("缺少停止范围")?;
                if !["all", "host", "group"].contains(&scope) {
                    return Err("停止范围无效".into());
                }
                self.stop(scope, value["id"].as_str()).await;
                Ok(self.status())
            }
            "shutdown" => {
                self.exit().await;
                Ok(json!({"stopped":true,"instance":self.instance}))
            }
            "preview" => {
                let h = self.host(&value)?;
                Ok(
                    serde_json::to_value(ssh::compile(&h, &self.root.join("runtime")).await?)
                        .unwrap(),
                )
            }
            "test" => {
                let h = self.host(&value)?;
                let snap = ssh::compile(&h, &self.root.join("runtime")).await?;
                let ms = ssh::latency(&snap).await;
                self.latency.lock().unwrap().insert(h.id.clone(), ms);
                Ok(json!({"hostId":h.id,"milliseconds":ms,"method":"SSH target TCP"}))
            }
            "answer" => {
                let id = value["id"].as_str().ok_or("缺少认证请求")?;
                let text = value["answer"].as_str().ok_or("缺少回答")?.to_owned();
                let p = self
                    .prompts
                    .lock()
                    .unwrap()
                    .remove(id)
                    .ok_or("认证请求已过期")?;
                p.answer.send(text).map_err(|_| "认证已取消")?;
                Ok(json!({"accepted":true}))
            }
            "cancelPrompt" => {
                let pid = value["id"].as_str().ok_or("缺少认证请求")?;
                let group = self
                    .prompts
                    .lock()
                    .unwrap()
                    .get(pid)
                    .map(|p| p.view.group_id.clone())
                    .ok_or("认证请求已过期")?;
                if group == "tools" {
                    let next = *self.tool_cancel.borrow() + 1;
                    self.tool_cancel.send_replace(next);
                    self.prompts
                        .lock()
                        .unwrap()
                        .retain(|_, p| p.view.group_id != "tools");
                } else {
                    self.stop("group", Some(&group)).await;
                }
                Ok(json!({"cancelled":true}))
            }
            "adopt" => {
                let gid = value["groupId"].as_str().ok_or("缺少连接组")?;
                let rule: Rule =
                    serde_json::from_value(value["rule"].clone()).map_err(|e| e.to_string())?;
                rule.validate()?;
                {
                    let gs = self.groups.lock().unwrap();
                    let g = gs.get(gid).ok_or("组不存在")?;
                    if !g.view.rules.iter().any(|r| r.same_intent(&rule)) {
                        return Err("保存的规则与运行转发不同".into());
                    }
                }
                let mut s = self.store.lock().unwrap();
                let mut c = s.config.clone();
                if c.rules.iter().any(|r| r.id == rule.id) {
                    return Err("规则 ID 已存在".into());
                }
                c.rules.push(rule.clone());
                s.save(c, value["revision"].as_str().ok_or("缺少配置版本")?)?;
                self.groups
                    .lock()
                    .unwrap()
                    .get_mut(gid)
                    .ok_or("组不存在")?
                    .view
                    .rules
                    .push(rule);
                Ok(json!({"saved":true}))
            }
            "files.read" | "files.write" | "keys.list" | "keys.generate" | "keys.public"
            | "keys.passphrase" | "keys.comment" | "agent.add" | "agent.remove"
            | "knownHosts.find" | "knownHosts.remove" | "observe" => {
                crate::tools::action(self, value).await
            }
            _ => Err("未知公共动作".into()),
        }
    }
    fn host(&self, v: &Value) -> Result<Host> {
        let mut h = self
            .store
            .lock()
            .unwrap()
            .config
            .hosts
            .iter()
            .find(|h| Some(h.id.as_str()) == v["hostId"].as_str())
            .cloned()
            .ok_or("主机不存在")?;
        if let Some(opts) = v["overrides"].as_array() {
            let mut o: Vec<String> = opts
                .iter()
                .map(|x| x.as_str().unwrap_or("").to_string())
                .collect();
            ssh::validate_options(&o)?;
            o.extend(h.options);
            h.options = o;
        }
        Ok(h)
    }
    pub async fn ensure(self: &Arc<Self>, mut request: Ensure) -> Result<Value> {
        let epoch = self.epoch.load(Ordering::SeqCst);
        let guard = self.gate.lock().await;
        if *self.shutdown.borrow() {
            return Err("后台正在退出".into());
        }
        let config = {
            let s = self.store.lock().unwrap();
            if let Some(e) = &s.error {
                return Err(format!("配置无法读取：{e}"));
            }
            s.config.clone()
        };
        if let Some(p) = &request.plan_id {
            let plan = config
                .plans
                .iter()
                .find(|x| &x.id == p)
                .ok_or("计划不存在")?;
            request.rule_ids.extend(plan.rule_ids.clone());
        }
        ssh::validate_options(&request.overrides)?;
        let mut requested = vec![];
        let mut results = vec![];
        let mut seen = HashSet::new();
        for rid in &request.rule_ids {
            if !seen.insert(rid.clone()) {
                continue;
            }
            match config.rules.iter().find(|r| &r.id == rid) {
                Some(r) => requested.push(r.clone()),
                None => results.push(json!({"ruleId":rid,"state":"failed","error":"规则不存在"})),
            }
        }
        for r in request.temporary {
            r.validate()?;
            if seen.insert(r.id.clone()) {
                requested.push(r);
            }
        }
        let mut snapshots = HashMap::new();
        let host_ids = requested
            .iter()
            .map(|r| r.host_id.clone())
            .collect::<HashSet<_>>();
        for hid in host_ids {
            let h = config.hosts.iter().find(|h| h.id == hid).cloned();
            let result = if let Some(mut h) = h {
                let mut o = request.overrides.clone();
                o.extend(h.options);
                h.options = o;
                ssh::compile(&h, &self.root.join("runtime")).await
            } else {
                Err("主机不存在".into())
            };
            snapshots.insert(hid, result);
        }
        // Source config forwards join the intent list; they never silently rebind an old port.
        for (hid, snapshot) in &snapshots {
            if let Ok(s) = snapshot {
                match inherited_rules(hid, &s.inherited_forwards) {
                    Ok(rs) => {
                        for r in rs {
                            if !requested.iter().any(|x| x.same_intent(&r)) {
                                requested.push(r);
                            }
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        }
        if epoch != self.epoch.load(Ordering::SeqCst) {
            return Err("启动已被停止操作取消".into());
        }
        let mut batches = HashMap::<String, (Snapshot, Vec<Rule>)>::new();
        let mut waits = vec![];
        for rule in requested {
            let snapshot = match snapshots.get(&rule.host_id).unwrap() {
                Ok(s) => s,
                Err(e) => {
                    results.push(json!({"ruleId":rule.id,"state":"failed","error":e}));
                    continue;
                }
            };
            let mut gs = self.groups.lock().unwrap();
            let existing = gs
                .values_mut()
                .find(|g| g.view.desired && g.view.rules.iter().any(|r| r.id == rule.id));
            if let Some(g) = existing {
                if g.view.snapshot.fingerprint != snapshot.fingerprint
                    || !g
                        .view
                        .rules
                        .iter()
                        .any(|r| r.id == rule.id && r.same_intent(&rule))
                {
                    results.push(json!({"ruleId":rule.id,"groupId":g.view.id,"state":"pending","error":"参数已变化；请显式停止连接组后重新启动"}));
                    continue;
                }
                add_consumer(&mut g.view, &request.consumer);
                waits.push((
                    rule.id.clone(),
                    g.view.id.clone(),
                    true,
                    g.status.subscribe(),
                ));
                continue;
            }
            if let Some(g) = gs.values_mut().find(|g| {
                g.view.desired
                    && g.view.snapshot.fingerprint == snapshot.fingerprint
                    && g.view.rules.iter().any(|r| r.same_intent(&rule))
            }) {
                g.view.rules.push(rule.clone());
                add_consumer(&mut g.view, &request.consumer);
                waits.push((
                    rule.id.clone(),
                    g.view.id.clone(),
                    true,
                    g.status.subscribe(),
                ));
                continue;
            }
            let has_conflict = gs
                .values()
                .any(|g| g.view.desired && g.view.rules.iter().any(|r| conflict(r, &rule)));
            if has_conflict {
                results.push(
                    json!({"ruleId":rule.id,"state":"conflict","error":"监听端与托管转发冲突"}),
                );
                continue;
            }
            drop(gs);
            if batches
                .values()
                .any(|(_, rs)| rs.iter().any(|r| conflict(r, &rule)))
            {
                results.push(json!({"ruleId":rule.id,"state":"conflict","error":"同批监听端重复"}));
                continue;
            }
            batches
                .entry(snapshot.fingerprint.clone())
                .or_insert_with(|| (snapshot.clone(), vec![]))
                .1
                .push(rule);
        }
        for (_, (snapshot, rules)) in batches {
            let gid = id();
            let file = self.root.join("runtime").join(format!("{gid}.conf"));
            store::atomic(&file, snapshot.config_text.as_bytes())?;
            let args = ssh::launch_args(&snapshot, &file, &rules);
            let (cancel, rx) = watch::channel(false);
            let (status, _) = watch::channel("starting".into());
            for rule in &rules {
                waits.push((rule.id.clone(), gid.clone(), false, status.subscribe()));
            }
            let mut g = Group {
                id: gid.clone(),
                host_id: snapshot.host.id.clone(),
                rules,
                consumers: vec![],
                state: "starting".into(),
                error: None,
                pid: None,
                attempts: 0,
                snapshot,
                args,
                logs: vec![],
                desired: true,
            };
            add_consumer(&mut g, &request.consumer);
            self.groups.lock().unwrap().insert(
                gid.clone(),
                Entry {
                    view: g,
                    cancel,
                    status,
                },
            );
            let rt = self.clone();
            tokio::spawn(async move {
                rt.supervise(gid, rx, file).await;
            });
        }
        drop(guard);
        for (rid, gid, reused, mut status) in waits {
            let result = tokio::time::timeout(Duration::from_secs(125), async {
                loop {
                    let s = status.borrow().clone();
                    if ["running", "failed", "stopped"].contains(&s.as_str()) {
                        return s;
                    }
                    if status.changed().await.is_err() {
                        return "failed".into();
                    }
                }
            })
            .await
            .unwrap_or("timeout".into());
            let error = self
                .groups
                .lock()
                .unwrap()
                .get(&gid)
                .and_then(|g| g.view.error.clone());
            results.push(
                json!({"ruleId":rid,"groupId":gid,"state":result,"reused":reused,"error":error}),
            );
        }
        Ok(
            json!({"instance":self.instance,"ready":!results.is_empty()&&results.iter().all(|r|r["state"]=="running"),"results":results}),
        )
    }
    fn update(&self, gid: &str, f: impl FnOnce(&mut Group)) {
        if let Some(e) = self.groups.lock().unwrap().get_mut(gid) {
            f(&mut e.view);
            e.status.send_replace(e.view.state.clone());
        }
    }
    async fn supervise(
        self: Arc<Self>,
        gid: String,
        mut cancel: watch::Receiver<bool>,
        file: PathBuf,
    ) {
        let snapshot = self.groups.lock().unwrap()[&gid].view.clone();
        let mut attempt = 0u32;
        loop {
            if *cancel.borrow() {
                break;
            }
            self.update(&gid, |g| {
                g.state = "starting".into();
                g.attempts = attempt;
                g.error = None;
            });
            let env = vec![
                (
                    "SSH_ASKPASS".into(),
                    std::env::current_exe()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                ),
                ("SSH_ASKPASS_REQUIRE".into(), "force".into()),
                ("DISPLAY".into(), "tether:0".into()),
                ("TETHER_ASKPASS_URL".into(), self.auth_origin.clone()),
                ("TETHER_ASKPASS_TOKEN".into(), self.auth_token.clone()),
                ("TETHER_ASKPASS_GROUP".into(), gid.clone()),
            ];
            let mut proc = match Owned::spawn(
                &snapshot.snapshot.host.ssh,
                &snapshot.args,
                &env,
                snapshot.snapshot.host.hidden,
            ) {
                Ok(p) => p,
                Err(e) => {
                    self.update(&gid, |g| {
                        g.state = "failed".into();
                        g.error = Some(e);
                        g.desired = false;
                    });
                    break;
                }
            };
            let pid = proc.child.id();
            self.update(&gid, |g| g.pid = pid);
            let stderr = proc.child.stderr.take().unwrap();
            let mut lines = BufReader::new(stderr).lines();
            let mut authenticated = false;
            let mut entered = false;
            let mut remote = 0;
            let remote_count = snapshot
                .rules
                .iter()
                .filter(|r| r.direction == Direction::Remote)
                .count();
            let mut local_ready = HashSet::<String>::new();
            let local_count = snapshot
                .rules
                .iter()
                .filter(|r| r.direction != Direction::Remote)
                .count();
            let mut running = false;
            let mut fatal = false;
            let mut error = String::new();
            let deadline =
                tokio::time::sleep(Duration::from_secs(snapshot.snapshot.host.timeout_secs));
            tokio::pin!(deadline);
            loop {
                tokio::select! {
                 biased;
                 _=cancel.changed()=>{if *cancel.borrow(){break;}},
                 _=&mut deadline,if !running=>{error="SSH 认证或转发就绪超时；可打开页面处理认证后重试".into();fatal=true;break;},
                 line=lines.next_line()=>{match line {Ok(Some(line))=>{
                  if line.contains("Local forwarding listening on ") {entered=false;for r in snapshot.rules.iter().filter(|r|r.direction!=Direction::Remote){if line.contains(&format!(" port {}.",r.port)) && (line.contains(&format!("on {} port",r.bind)) || wildcard(&r.bind) || r.bind=="localhost" && (line.contains("on 127.0.0.1 port")||line.contains("on ::1 port"))){local_ready.insert(r.id.clone());}}}
                  authenticated|=line.contains("Authenticated to ")||line.contains("Authentication succeeded (");entered|=line.contains("Entering interactive session.");if line.contains("remote forward success for:"){remote+=1;}
                  if ["Permission denied","REMOTE HOST IDENTIFICATION HAS CHANGED","Host key verification failed","Bad configuration","Bad owner or permissions","Address already in use","Could not request local forwarding","remote port forwarding failed","Bad local forwarding","Bad remote forwarding","Bad dynamic forwarding","no matching "].iter().any(|s|line.contains(s)){fatal=true;error=line.clone();}
                  // Native DEBUG1 is reserved for readiness. Never log askpass answers.
                  self.update(&gid,|g|{g.logs.push(line.clone());if g.logs.len()>160{g.logs.remove(0);}});
                  if fatal{break;}
                  if !running && authenticated && entered && remote>=remote_count && local_ready.len()>=local_count {running=true;self.update(&gid,|g|g.state="running".into());let rt=self.clone();let snap=snapshot.snapshot.clone();tokio::spawn(async move{let ms=ssh::latency(&snap).await;rt.latency.lock().unwrap().insert(snap.host.id,ms);});}
                 },Ok(None)=>{break;},Err(e)=>{error=e.to_string();break;}}},
                }
            }
            proc.stop().await;
            self.prompts
                .lock()
                .unwrap()
                .retain(|_, p| p.view.group_id != gid);
            if *cancel.borrow() {
                break;
            }
            if error.is_empty() {
                error = "SSH 连接已断开".into();
            }
            if fatal || !snapshot.snapshot.host.reconnect || !running && attempt >= 4 {
                self.update(&gid, |g| {
                    g.state = "failed".into();
                    g.desired = false;
                    g.error = Some(error.clone());
                    g.pid = None;
                });
                break;
            }
            attempt = if running { 1 } else { attempt + 1 };
            let delay = (1u64 << attempt.min(5)) + u64::from(gid.as_bytes()[0] % 3);
            self.update(&gid, |g| {
                g.state = "reconnecting".into();
                g.error = Some(error);
                g.pid = None;
            });
            tokio::select! {_=cancel.changed()=>{},_=tokio::time::sleep(Duration::from_secs(delay))=>{}}
        }
        if *cancel.borrow() {
            self.update(&gid, |g| {
                g.state = "stopped".into();
                g.desired = false;
                g.pid = None;
            });
        }
        let _ = std::fs::remove_file(file);
    }
    #[cfg(windows)]
    pub fn owned_pids(&self) -> Vec<u32> {
        self.groups
            .lock()
            .unwrap()
            .values()
            .filter_map(|g| g.view.pid)
            .collect()
    }
    pub async fn exit(&self) {
        self.shutdown.send_replace(true);
        let next = *self.tool_cancel.borrow() + 1;
        self.tool_cancel.send_replace(next);
        self.prompts.lock().unwrap().clear();
        self.stop("all", None).await;
        let _ = self.shutdown.send(true);
    }
    pub async fn stop(&self, scope: &str, id: Option<&str>) {
        self.epoch.fetch_add(1, Ordering::SeqCst);
        let mut statuses = vec![];
        {
            let mut gs = self.groups.lock().unwrap();
            for e in gs.values_mut().filter(|e| {
                scope == "all"
                    || scope == "host" && Some(e.view.host_id.as_str()) == id
                    || scope == "group" && Some(e.view.id.as_str()) == id
            }) {
                if e.view.desired {
                    e.view.desired = false;
                    let _ = e.cancel.send(true);
                    e.view.state = "stopping".into();
                    statuses.push(e.status.subscribe());
                }
            }
        }
        for mut rx in statuses {
            let _ = tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    if ["stopped", "failed"].contains(&rx.borrow().as_str()) {
                        break;
                    }
                    if rx.changed().await.is_err() {
                        break;
                    }
                }
            })
            .await;
        }
    }
    pub async fn prompt(&self, gid: &str, text: String) -> Result<String> {
        let host = if gid == "tools" {
            "密钥 / Agent 工具".into()
        } else {
            let gs = self.groups.lock().unwrap();
            let g = gs.get(gid).ok_or("未知连接组")?;
            if !g.view.desired {
                return Err("连接已取消".into());
            }
            g.view.snapshot.host.name.clone()
        };
        let secret = !(text.to_lowercase().contains("yes/no")
            || text.to_lowercase().contains("fingerprint"));
        let prompt = Prompt {
            id: id(),
            group_id: gid.into(),
            host,
            text,
            secret,
        };
        let pid = prompt.id.clone();
        let (tx, rx) = oneshot::channel();
        self.prompts.lock().unwrap().insert(
            pid.clone(),
            Pending {
                view: prompt,
                answer: tx,
            },
        );
        self.update(gid, |g| g.state = "authentication".into());
        let result = tokio::time::timeout(Duration::from_secs(110), rx)
            .await
            .map_err(|_| "认证回答超时".to_string())
            .and_then(|x| x.map_err(|_| "认证已取消".to_string()));
        self.prompts.lock().unwrap().remove(&pid);
        self.update(gid, |g| {
            if g.desired {
                g.state = "starting".into();
            }
        });
        result
    }
}
fn add_consumer(g: &mut Group, c: &str) {
    if !c.is_empty() && !g.consumers.iter().any(|x| x == c) {
        g.consumers.push(c.into());
    }
}
fn wildcard(s: &str) -> bool {
    ["", "*", "0.0.0.0", "::"].contains(&s)
}
fn conflict(a: &Rule, b: &Rule) -> bool {
    let local_a = a.direction != Direction::Remote;
    let local_b = b.direction != Direction::Remote;
    local_a == local_b
        && (local_a || a.host_id == b.host_id)
        && a.port == b.port
        && (a.bind == b.bind
            || wildcard(&a.bind)
            || wildcard(&b.bind)
            || a.bind == "localhost"
            || b.bind == "localhost")
}
fn endpoint(s: &str) -> Result<(String, u16)> {
    let (host, port) = s
        .rsplit_once(':')
        .ok_or("不支持 SSH Unix socket 转发；请使用 TCP 规则")?;
    let port = port.parse().map_err(|_| "转发端口格式无效")?;
    Ok((host.trim_matches(['[', ']']).into(), port))
}
fn inherited_rules(hid: &str, lines: &[String]) -> Result<Vec<Rule>> {
    lines
        .iter()
        .map(|line| {
            let words = line.split_whitespace().collect::<Vec<_>>();
            let kind = *words.first().ok_or("空转发")?;
            let (bind, port) = endpoint(words.get(1).ok_or("转发缺少监听地址")?)?;
            let (direction, target, target_port) = match kind {
                "dynamicforward" => (Direction::Dynamic, String::new(), 0),
                "localforward" | "remoteforward" => {
                    let (t, p) = endpoint(
                        words
                            .get(2)
                            .ok_or("配置中的动态远程转发暂不支持，请使用固定 TCP 目标")?,
                    )?;
                    (
                        if kind == "localforward" {
                            Direction::Local
                        } else {
                            Direction::Remote
                        },
                        t,
                        p,
                    )
                }
                _ => return Err("未知转发来源".into()),
            };
            Ok(Rule {
                id: format!("ssh:{hid}:{}", &store::revision(line.as_bytes())[..16]),
                name: format!("SSH config · {port}"),
                host_id: hid.into(),
                direction,
                bind,
                port,
                target,
                target_port,
                notes: "来自 SSH 生效配置（只读）".into(),
            })
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_port_never_implies_ownership() {
        let rs = inherited_rules(
            "h",
            &[
                "localforward [127.0.0.1]:80 [db]:90".into(),
                "dynamicforward [::1]:1080".into(),
            ],
        )
        .unwrap();
        assert_eq!(rs[0].target, "db");
        assert_eq!(rs[1].bind, "::1");
        assert!(!conflict(&rs[0], &rs[1]));
        let mut d = rs[0].clone();
        d.direction = Direction::Dynamic;
        assert!(conflict(&rs[0], &d));
    }
    #[test]
    fn rejects_unsupported_source_semantics() {
        assert!(inherited_rules("h", &["localforward /tmp/sock /tmp/target".into()]).is_err());
    }
}
