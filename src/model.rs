use serde::{Deserialize, Serialize};
use std::collections::HashSet;
pub type Result<T> = std::result::Result<T, String>;
pub fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}
fn ssh() -> String {
    "ssh".into()
}
fn timeout() -> u64 {
    30
}
fn yes() -> bool {
    true
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Host {
    pub id: String,
    pub name: String,
    pub destination: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub category: String,
    #[serde(default = "ssh")]
    pub ssh: String,
    #[serde(default)]
    pub config_file: Option<String>,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default = "yes")]
    pub reconnect: bool,
    #[serde(default = "yes")]
    pub hidden: bool,
    #[serde(default = "timeout")]
    pub timeout_secs: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Local,
    Remote,
    Dynamic,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub name: String,
    pub host_id: String,
    pub direction: Direction,
    pub bind: String,
    pub port: u16,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub target_port: u16,
    #[serde(default)]
    pub notes: String,
    #[serde(default, rename = "openUrl", skip_serializing)]
    pub legacy_open_url: serde_json::Value,
}
impl Rule {
    pub fn same_intent(&self, other: &Self) -> bool {
        self.host_id == other.host_id
            && self.direction == other.direction
            && self.bind == other.bind
            && self.port == other.port
            && self.target == other.target
            && self.target_port == other.target_port
    }
    pub fn endpoint(&self) -> String {
        format!("{}:{}", bracket(&self.bind), self.port)
    }
    pub fn spec(&self) -> String {
        if self.direction == Direction::Dynamic {
            self.endpoint()
        } else {
            format!(
                "{}:{}:{}",
                self.endpoint(),
                bracket(&self.target),
                self.target_port
            )
        }
    }
    pub fn validate(&self) -> Result<()> {
        valid_id(&self.id)?;
        valid_id(&self.host_id)?;
        if self.name.trim().is_empty() || self.port == 0 {
            return Err("规则名称和非零端口必填".into());
        }
        address(&self.bind, true)?;
        if self.direction != Direction::Dynamic {
            address(&self.target, false)?;
            if self.target_port == 0 {
                return Err("目标端口须非零".into());
            }
        }
        Ok(())
    }
}
pub fn bracket(s: &str) -> String {
    if s.contains(':') {
        format!("[{s}]")
    } else {
        s.into()
    }
}
fn address(s: &str, wild: bool) -> Result<()> {
    if (!wild && s.is_empty())
        || s.chars().any(|c| {
            c.is_whitespace() || matches!(c, '\n' | '\r' | '"' | '\'' | '[' | ']' | '/' | '\\')
        })
    {
        Err("地址格式无效（IPv6 不带方括号）".into())
    } else {
        Ok(())
    }
}
pub fn valid_id(s: &str) -> Result<()> {
    if s.is_empty()
        || s.len() > 128
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-:".contains(&b))
    {
        Err("稳定 ID 无效".into())
    } else {
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plan {
    pub id: String,
    pub name: String,
    pub rule_ids: Vec<String>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub auto_start: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    #[serde(default)]
    pub hosts: Vec<Host>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub plans: Vec<Plan>,
    #[serde(default)]
    pub key_files: Vec<String>,
    #[serde(default)]
    pub ssh_files: Vec<String>,
    #[serde(default)]
    pub theme: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_socket: Option<String>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            hosts: vec![],
            rules: vec![],
            plans: vec![],
            key_files: vec![],
            ssh_files: vec![],
            theme: "system".into(),
            agent_socket: None,
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        if self
            .agent_socket
            .as_ref()
            .is_some_and(|p| !std::path::Path::new(p).is_absolute())
        {
            return Err("Agent socket 须为当前环境的绝对路径".into());
        }
        if self.version != 1 {
            return Err("不支持的配置版本，原文件保留".into());
        }
        if self
            .key_files
            .iter()
            .chain(&self.ssh_files)
            .any(|p| !std::path::Path::new(p).is_absolute())
        {
            return Err("密钥和 SSH 文件引用须为本机绝对路径".into());
        }
        let mut ids = HashSet::new();
        for h in &self.hosts {
            valid_id(&h.id)?;
            if !ids.insert(&h.id)
                || h.name.trim().is_empty()
                || h.destination.is_empty()
                || h.destination.starts_with('-')
                || h.destination.chars().any(|c| {
                    c.is_whitespace() || matches!(c, '"' | '\'' | '*' | '?' | '!' | '[' | ']')
                })
                || !(1..=120).contains(&h.timeout_secs)
            {
                return Err("主机 ID、目标或超时无效".into());
            }
            crate::ssh::validate_options(&h.options)?;
            let target_user = h
                .destination
                .strip_prefix("ssh://")
                .unwrap_or(&h.destination)
                .split_once('@')
                .map(|(user, _)| user);
            if let Some(target_user) = target_user {
                for option in &h.options {
                    if crate::ssh::key(option) == "user" {
                        let user = option
                            .split_once('=')
                            .map(|(_, v)| v.trim().trim_matches('"'))
                            .unwrap_or("");
                        if user != target_user {
                            return Err("SSH 目标中的用户与 User 覆盖冲突，请只保留一个来源".into());
                        }
                    }
                }
            }
        }
        let mut rules = HashSet::new();
        for r in &self.rules {
            r.validate()?;
            if !rules.insert(&r.id) || !ids.contains(&r.host_id) {
                return Err("规则 ID 重复或主机不存在".into());
            }
        }
        let mut plans = HashSet::new();
        for p in &self.plans {
            valid_id(&p.id)?;
            if p.name.trim().is_empty()
                || !plans.insert(&p.id)
                || p.rule_ids.iter().any(|r| !rules.contains(r))
                || p.rule_ids.iter().collect::<HashSet<_>>().len() != p.rule_ids.len()
            {
                return Err("计划 ID 重复或规则引用无效".into());
            }
        }
        if !["system", "light", "dark"].contains(&self.theme.as_str()) {
            return Err("主题无效".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn config() -> Config {
        serde_json::from_value(serde_json::json!({
            "version":1,"hosts":[{"id":"h","name":"主机","destination":"admin@fixture"}],
            "rules":[{"id":"r","hostId":"h","name":"Web","direction":"local","bind":"127.0.0.1","port":18080,"target":"127.0.0.1","targetPort":8080}],
            "plans":[{"id":"p","name":"计划","ruleIds":["r"]}],"theme":"system"
        })).unwrap()
    }
    #[test]
    fn old_configs_drop_web_shortcuts_and_keep_stable_references() {
        let mut c = config();
        assert!(c.validate().is_ok());
        let mut old = serde_json::to_value(&c).unwrap();
        old["rules"][0]["openUrl"] = serde_json::json!("https://old.example/");
        c = serde_json::from_value(old).unwrap();
        assert!(c.validate().is_ok());
        assert!(serde_json::to_value(&c).unwrap()["rules"][0]
            .get("openUrl")
            .is_none());
        let roundtrip: Config = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(roundtrip.plans[0].rule_ids, vec!["r"]);
        assert_eq!(roundtrip.hosts[0].id, "h");
    }
    #[test]
    fn rejects_conflicting_user() {
        let mut c = config();
        c.hosts[0].options.push("User=other".into());
        assert!(c.validate().unwrap_err().contains("用户"));
        c.hosts[0].options = vec!["User=admin".into()];
        assert!(c.validate().is_ok());
    }
}
