use crate::{model::*, platform, store};
use fs2::FileExt;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct Paths {
    pub data: PathBuf,
    pub config: PathBuf,
    pub runtime: PathBuf,
}
impl Paths {
    pub fn prepare(args: &[String]) -> Result<Self> {
        let explicit = args
            .windows(2)
            .find(|a| a[0] == "--data-dir")
            .map(|a| PathBuf::from(&a[1]))
            .or_else(|| std::env::var_os("TETHER_DATA_DIR").map(PathBuf::from));
        let portable = explicit.is_none();
        let data = explicit.unwrap_or(
            std::env::current_exe()
                .map_err(|e| e.to_string())?
                .parent()
                .ok_or("程序路径无父目录")?
                .join("data"),
        );
        if !data.is_absolute() {
            return Err("数据目录须使用绝对路径".into());
        }
        store::private_dir(&data).map_err(|e| format!("数据目录不可写 {}：{e}", data.display()))?;
        let config = if portable {
            data.join(platform::local_id())
        } else {
            data.clone()
        };
        store::private_dir(&config)?;
        // Legacy instances hold this lock for their whole lifetime. Never terminate them.
        check_legacy(&config)?;
        if portable {
            #[cfg(windows)]
            let old = PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("缺少 LOCALAPPDATA")?)
                .join("Tether");
            #[cfg(not(windows))]
            let old = data.join("no-legacy-linux-config");
            migrate(&old, &config)?;
        }
        // Check the actual source directory, including when a valid config already exists.
        let probe = data.join(format!(".write-{}", id()));
        store::atomic(&probe, b"")
            .map_err(|e| format!("数据目录不可写 {}：{e}", data.display()))?;
        fs::remove_file(probe).map_err(|e| e.to_string())?;
        let runtime = platform::runtime_dir(&config)?;
        Ok(Self {
            data,
            config,
            runtime,
        })
    }
}
fn check_legacy(dir: &Path) -> Result<()> {
    let path = dir.join("instance.lock");
    if path.exists() {
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(|e| e.to_string())?;
        lock.try_lock_exclusive().map_err(|_| {
            "旧后台仍在运行，请先正常退出旧后台后重试；已有连接保持不动".to_string()
        })?;
    }
    Ok(())
}

fn migrate(old: &Path, config: &Path) -> Result<()> {
    check_legacy(old)?;
    if config.join("config.json").exists() {
        return Ok(());
    }
    let bytes = if old.join("config.json").exists() {
        let b = store::read(&old.join("config.json"))?;
        let c: Config = serde_json::from_slice(&b).map_err(|e| format!("旧配置无法迁入：{e}"))?;
        c.validate()?;
        b
    } else {
        serde_json::to_vec_pretty(&Config::default()).unwrap()
    };
    store::atomic(&config.join("config.json"), &bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_preserves_original_ids_and_never_reimports() {
        let root = std::env::temp_dir().join(id());
        let old = root.join("old");
        let new = root.join("portable");
        store::private_dir(&old).unwrap();
        store::private_dir(&new).unwrap();
        let text=br#"{"version":1,"hosts":[{"id":"stable-host","name":"Host","destination":"fixture"}],"rules":[],"plans":[],"theme":"system"}"#;
        fs::write(old.join("config.json"), text).unwrap();
        migrate(&old, &new).unwrap();
        assert_eq!(fs::read(new.join("config.json")).unwrap(), text);
        assert_eq!(fs::read(old.join("config.json")).unwrap(), text);
        fs::write(old.join("config.json"), b"broken").unwrap();
        migrate(&old, &new).unwrap();
        assert_eq!(fs::read(new.join("config.json")).unwrap(), text);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn migration_requires_legacy_instance_to_exit() {
        let root = std::env::temp_dir().join(id());
        store::private_dir(&root).unwrap();
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(root.join("instance.lock"))
            .unwrap();
        lock.lock_exclusive().unwrap();
        assert!(check_legacy(&root).unwrap_err().contains("正常退出"));
        drop(lock);
        check_legacy(&root).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
