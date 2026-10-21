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
        }
    }
}
impl Config {
    pub fn validate(&self) -> Result<()> {
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
            if !plans.insert(&p.id)
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
