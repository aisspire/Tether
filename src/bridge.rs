//! Multiplexed private stdio protocol. Replies carry IDs and stdout contains no diagnostics.
use crate::{model::*, server};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin},
    sync::oneshot,
};
pub const PROTOCOL: u32 = 1;
type Pending = Arc<Mutex<HashMap<String, oneshot::Sender<Result<Value>>>>>;
pub struct Bridge {
    input: tokio::sync::Mutex<ChildStdin>,
    child: tokio::sync::Mutex<Child>,
    pending: Pending,
    next: AtomicU64,
    alive: Arc<std::sync::atomic::AtomicBool>,
}
impl Bridge {
    pub async fn connect(mut child: Child, expected: &str) -> Result<Arc<Self>> {
        let input = child.stdin.take().ok_or("WSL 标准输入不可用")?;
        let stdout = child.stdout.take().ok_or("WSL 标准输出不可用")?;
        let mut lines = BufReader::new(stdout).lines();
        let line = tokio::time::timeout(Duration::from_secs(25), lines.next_line())
            .await
            .map_err(|_| "WSL 后台启动超时")?
            .map_err(|e| e.to_string())?
            .ok_or("WSL 后台启动失败，请检查随包 Linux 程序及发行版权限")?;
        let descriptor: Value = serde_json::from_str(&line).map_err(|_| "WSL 握手无效")?;
        if descriptor["bridgeProtocol"] != PROTOCOL
            || descriptor["version"] != env!("CARGO_PKG_VERSION")
            || descriptor["environment"] != expected
        {
            let _ = child.kill().await;
            return Err("Windows / Linux 后台版本或环境身份不匹配，请使用同一目录包".into());
        }
        let pending: Pending = Arc::default();
        let alive = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let p = pending.clone();
        let a = alive.clone();
        tokio::spawn(async move {
            while let Ok(Some(line)) = lines.next_line().await {
                if line.len() > 8 * 1024 * 1024 {
                    break;
                }
                let Ok(v) = serde_json::from_str::<Value>(&line) else {
                    break;
                };
                let Some(id) = v["id"].as_str() else {
                    break;
                };
                if let Some(tx) = p.lock().unwrap().remove(id) {
                    let result = if v["ok"] == true {
                        Ok(v["value"].clone())
                    } else {
                        Err(v["error"].as_str().unwrap_or("WSL 动作失败").into())
                    };
                    let _ = tx.send(result);
                }
            }
            a.store(false, Ordering::SeqCst);
            for (_, tx) in p.lock().unwrap().drain() {
                let _ = tx.send(Err(
                    "WSL 通信中断；后台会清理其托管连接，请重新选择环境".into()
                ));
            }
        });
        Ok(Arc::new(Self {
            input: tokio::sync::Mutex::new(input),
            child: tokio::sync::Mutex::new(child),
            pending,
            next: AtomicU64::new(0),
            alive,
        }))
    }
    pub fn alive(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }
    pub async fn call(&self, request: Value) -> Result<Value> {
        if !self.alive() {
            return Err("WSL 后台已断开，请重新选择环境".into());
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed).to_string();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id.clone(), tx);
        let frame = json!({"id":id,"request":request});
        let written = {
            let mut input = self.input.lock().await;
            match input.write_all(format!("{frame}\n").as_bytes()).await {
                Ok(()) => input.flush().await,
                Err(e) => Err(e),
            }
        };
        if let Err(e) = written {
            self.pending.lock().unwrap().remove(&id);
            return Err(e.to_string());
        }
        let result = tokio::time::timeout(Duration::from_secs(135), rx).await;
        self.pending.lock().unwrap().remove(&id);
        result
            .map_err(|_| "WSL 动作超时，可查询状态或取消".to_string())?
            .map_err(|_| "WSL 通信已关闭".to_string())?
    }
    pub async fn close(&self) {
        if self.alive() {
            let _ = tokio::time::timeout(
                Duration::from_secs(15),
                self.call(json!({"action":"shutdown"})),
            )
            .await;
        }
        let _ = self.input.lock().await.shutdown().await;
        let mut child = self.child.lock().await;
        if tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .is_err()
        {
            let _ = child.kill().await;
        }
    }
}
pub async fn write(
    output: &tokio::sync::Mutex<tokio::io::Stdout>,
    id: Option<Value>,
    result: Result<Value>,
) -> Result<()> {
    let mut frame = server::envelope(result);
    if let Some(id) = id {
        frame["id"] = id;
    }
    let mut out = output.lock().await;
    out.write_all(format!("{frame}\n").as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    out.flush().await.map_err(|e| e.to_string())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::process::Stdio;
    #[tokio::test]
    async fn incompatible_handshake_is_rejected() {
        let mut cmd = crate::platform::command("sh");
        cmd.args([
            "-c",
            "printf '%s\\n' '{\"bridgeProtocol\":999,\"environment\":\"fixture\"}'",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
        let err = Bridge::connect(cmd.spawn().unwrap(), "fixture")
            .await
            .err()
            .unwrap();
        assert!(err.contains("版本") || err.contains("握手"));
    }
    #[tokio::test]
    async fn disconnect_rejects_concurrent_requests() {
        let greeting=json!({"bridgeProtocol":PROTOCOL,"environment":"fixture","version":env!("CARGO_PKG_VERSION")}).to_string();
        let mut cmd = crate::platform::command("sh");
        cmd.args([
            "-c",
            "printf '%s\\n' \"$1\"; read line",
            "fixture",
            &greeting,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
        let bridge = Bridge::connect(cmd.spawn().unwrap(), "fixture")
            .await
            .unwrap();
        let (a, b) = tokio::join!(
            bridge.call(json!({"action":"status"})),
            bridge.call(json!({"action":"status"}))
        );
        assert!(a.is_err());
        assert!(b.is_err());
        bridge.close().await;
    }
}
