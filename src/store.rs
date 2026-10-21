use crate::model::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
pub fn revision(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn read(path: &Path) -> Result<Vec<u8>> {
    let m = fs::metadata(path).map_err(|e| e.to_string())?;
    if m.len() > 2 * 1024 * 1024 {
        return Err("文件超过 2 MiB".into());
    }
    fs::read(path).map_err(|e| e.to_string())
}
pub fn private_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or("无效目录")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = parent.join(format!(".{}.tmp", id()));
    let result = (|| {
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&temp).map_err(|e| e.to_string())?;
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?;
        drop(f);
        fs::rename(&temp, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
pub struct Store {
    pub path: PathBuf,
    pub config: Config,
    pub error: Option<String>,
    pub revision: String,
}
impl Store {
    pub fn open(root: &Path) -> Self {
        let path = root.join("config.json");
        let bytes = if path.exists() {
            read(&path)
        } else {
            Ok(vec![])
        };
        let rev = bytes.as_ref().map(|b| revision(b)).unwrap_or_default();
        let result = bytes
            .and_then(|b| {
                if b.is_empty() && !path.exists() {
                    Ok(Config::default())
                } else {
                    serde_json::from_slice::<Config>(&b).map_err(|e| e.to_string())
                }
            })
            .and_then(|c| {
                c.validate()?;
                Ok(c)
            });
        match result {
            Ok(config) => Self {
                path,
                config,
                error: None,
                revision: rev,
            },
            Err(e) => Self {
                path,
                config: Config::default(),
                error: Some(e),
                revision: rev,
            },
        }
    }
    pub fn save(&mut self, config: Config, expected: &str) -> Result<()> {
        if self.error.is_some() {
            return Err("配置读取失败；修复文件后重新加载".into());
        }
        config.validate()?;
        let current = if self.path.exists() {
            read(&self.path)?
        } else {
            vec![]
        };
        if expected != self.revision || revision(&current) != self.revision {
            return Err("配置已在外部变化，请重新加载".into());
        }
        let bytes = serde_json::to_vec_pretty(&config).map_err(|e| e.to_string())?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err("配置超过 2 MiB".into());
        }
        atomic(&self.path, &bytes)?;
        self.revision = revision(&bytes);
        self.config = config;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserve_corrupt_and_external_edits() {
        let dir = std::env::temp_dir().join(id());
        private_dir(&dir).unwrap();
        fs::write(dir.join("config.json"), b"broken").unwrap();
        let mut s = Store::open(&dir);
        assert!(s.save(Config::default(), &s.revision.clone()).is_err());
        assert_eq!(fs::read(dir.join("config.json")).unwrap(), b"broken");
        fs::remove_file(dir.join("config.json")).unwrap();
        let mut s = Store::open(&dir);
        let rev = s.revision.clone();
        s.save(Config::default(), &rev).unwrap();
        fs::write(&s.path, b"external").unwrap();
        assert!(s.save(Config::default(), &s.revision.clone()).is_err());
        assert_eq!(fs::read(&s.path).unwrap(), b"external");
        fs::remove_dir_all(dir).unwrap();
    }
}
