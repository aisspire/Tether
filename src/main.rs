mod external;
mod model;
mod owned;
mod runtime;
mod server;
mod ssh;
mod store;
mod tools;
use model::*;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use tokio::io::AsyncWriteExt;
fn root(args: &[String]) -> PathBuf {
    args.windows(2)
        .find(|a| a[0] == "--data-dir")
        .map(|a| PathBuf::from(&a[1]))
        .or_else(|| std::env::var_os("TETHER_DATA_DIR").map(PathBuf::from))
        .unwrap_or_else(|| {
            #[cfg(windows)]
            {
                PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("Tether")
            }
            #[cfg(not(windows))]
            {
                PathBuf::from(
                    std::env::var_os("XDG_CONFIG_HOME")
                        .unwrap_or_else(|| std::env::temp_dir().join("tether-validation").into()),
                )
                .join("tether")
            }
        })
}
#[tokio::main]
async fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let result = run(args).await;
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
async fn run(args: Vec<String>) -> Result<()> {
    // OpenSSH invokes the same executable with the prompt as its single argument.
    if let Ok(origin) = std::env::var("TETHER_ASKPASS_URL") {
        let token = std::env::var("TETHER_ASKPASS_TOKEN").map_err(|_| "缺少认证身份")?;
        let response=server::call(&origin,"/prompt",&token,json!({"groupId":std::env::var("TETHER_ASKPASS_GROUP").unwrap_or_default(),"text":args.join(" ")})).await?;
        println!("{}", response["answer"].as_str().ok_or("认证已取消")?);
        return Ok(());
    }
    let root = root(&args);
    if !root.is_absolute() {
        return Err("数据目录须使用绝对路径".into());
    }
    match args.first().map(|s| s.as_str()).unwrap_or("help") {
        "serve" => serve(root, args.iter().any(|a| a == "--oriel-stdio")).await,
        "request" => {
            let desc: Value = serde_json::from_slice(&store::read(&root.join("instance.json"))?)
                .map_err(|e| e.to_string())?;
            let body = args
                .windows(2)
                .find(|a| a[0] == "--json")
                .map(|a| a[1].as_str())
                .ok_or("用法：tether request --json '{\"action\":\"status\"}'")?;
            let body = serde_json::from_str(body).map_err(|e| e.to_string())?;
            let out = server::call(
                desc["origin"].as_str().ok_or("无控制地址")?,
                "/api",
                desc["token"].as_str().ok_or("无实例身份")?,
                body,
            )
            .await?;
            println!("{out}");
            if out["ok"] != true {
                return Err("动作失败".into());
            }
            Ok(())
        }
        "check" => {
            let mut cmd = ssh::command("ssh");
            cmd.arg("-V");
            let out = cmd
                .output()
                .await
                .map_err(|e| format!("缺少本机 OpenSSH：{e}"))?;
            if !out.status.success() {
                return Err("OpenSSH 检查失败".into());
            }
            println!(
                "{}",
                json!({"protocol":1,"ssh":String::from_utf8_lossy(&out.stderr),"version":env!("CARGO_PKG_VERSION")})
            );
            Ok(())
        }
        _ => {
            println!("Tether {}\n  serve [--data-dir ABSOLUTE] [--oriel-stdio]\n  request [--data-dir ABSOLUTE] --json JSON\n  check\nWindows 原生 OpenSSH；serve 返回独立页面 URL，按 Ctrl+C 停止托管连接。",env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
async fn serve(root: PathBuf, stdio: bool) -> Result<()> {
    use fs2::FileExt;
    store::private_dir(&root)?;
    store::private_dir(&root.join("runtime"))?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(root.join("instance.lock"))
        .map_err(|e| e.to_string())?;
    if lock.try_lock_exclusive().is_err() {
        if stdio {
            return proxy(root).await;
        }
        return Err("此配置已有后台实例；请通过 request 或 instance.json 中的页面地址连接".into());
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let auth = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let auth_origin = format!("http://{}", auth.local_addr().unwrap());
    let token = format!("{}{}", id(), id());
    let rt = runtime::Runtime::new(root.clone(), origin.clone(), token.clone(), auth_origin);
    let descriptor = json!({"protocol":1,"instance":rt.instance,"origin":origin,"token":token,"pageUrl":format!("{origin}/#token={token}"),"ready":true});
    store::atomic(
        &root.join("instance.json"),
        &serde_json::to_vec(&descriptor).unwrap(),
    )?;
    let mut out = tokio::io::stdout();
    out.write_all(format!("{descriptor}\n").as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    out.flush().await.map_err(|e| e.to_string())?;
    tokio::spawn(server::auth(rt.clone(), auth));
    if stdio {
        let (tx, mut requests) = tokio::sync::mpsc::channel::<String>(32);
        let reader_rt = rt.clone();
        let handle = tokio::runtime::Handle::current();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let input = std::io::stdin();
            for line in std::io::BufReader::new(input.lock()).lines() {
                match line {
                    Ok(line) if line.len() <= 2 * 1024 * 1024 => {
                        if tx.blocking_send(line).is_err() {
                            break;
                        }
                    }
                    _ => break,
                }
            }
            handle.spawn(async move {
                reader_rt.exit().await;
                drop(tx);
            });
        });
        let action_rt = rt.clone();
        tokio::spawn(async move {
            while let Some(line) = requests.recv().await {
                if *action_rt.shutdown.borrow() {
                    break;
                }
                let result = match serde_json::from_str::<Value>(&line) {
                    Ok(v) => action_rt.action(v).await,
                    Err(e) => Err(e.to_string()),
                };
                let response = server::envelope(result);
                let mut out = tokio::io::stdout();
                if out
                    .write_all(format!("{response}\n").as_bytes())
                    .await
                    .is_err()
                {
                    action_rt.exit().await;
                    break;
                }
                let _ = out.flush().await;
            }
        });
    }
    let plans = rt
        .store
        .lock()
        .unwrap()
        .config
        .plans
        .iter()
        .filter(|p| p.auto_start)
        .map(|p| p.id.clone())
        .collect::<Vec<_>>();
    for p in plans {
        let rt = rt.clone();
        tokio::spawn(async move {
            let _ = rt
                .ensure(runtime::Ensure {
                    plan_id: Some(p),
                    consumer: "auto-start".into(),
                    ..Default::default()
                })
                .await;
        });
    }
    let mut shutdown = rt.shutdown.subscribe();
    let stopped = rt.clone();
    axum::serve(listener, server::router(rt.clone()))
        .with_graceful_shutdown(async move {
            tokio::select! {_=tokio::signal::ctrl_c()=>{},_=shutdown.changed()=>{}}
            stopped.exit().await;
        })
        .await
        .map_err(|e| e.to_string())?;
    let _ = fs::remove_file(root.join("instance.json"));
    drop(lock);
    Ok(())
}

async fn proxy(root: PathBuf) -> Result<()> {
    let descriptor: Value = serde_json::from_slice(&store::read(&root.join("instance.json"))?)
        .map_err(|e| e.to_string())?;
    let origin = descriptor["origin"].as_str().ok_or("实例无控制地址")?;
    let token = descriptor["token"].as_str().ok_or("实例无调用身份")?;
    let status = server::call(origin, "/api", token, json!({"action":"status"})).await?;
    if status["ok"] != true || status["value"]["instance"] != descriptor["instance"] {
        return Err("已有实例身份校验失败".into());
    }
    println!("{descriptor}");
    let (tx, mut requests) = tokio::sync::mpsc::channel::<String>(32);
    let owned_origin = origin.to_string();
    let owned_token = token.to_string();
    let handle = tokio::runtime::Handle::current();
    std::thread::spawn(move || {
        use std::io::BufRead;
        let input = std::io::stdin();
        for line in std::io::BufReader::new(input.lock()).lines() {
            match line {
                Ok(line) if line.len() <= 2 * 1024 * 1024 => {
                    if tx.blocking_send(line).is_err() {
                        break;
                    }
                }
                _ => break,
            }
        }
        handle.spawn(async move {
            let _ = server::call(
                &owned_origin,
                "/api",
                &owned_token,
                json!({"action":"shutdown"}),
            )
            .await;
            drop(tx);
        });
    });
    while let Some(line) = requests.recv().await {
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(v) => server::call(origin, "/api", token, v).await,
            Err(e) => Err(e.to_string()),
        };
        match response {
            Ok(v) => println!("{v}"),
            Err(e) => println!("{}", server::envelope(Err(e))),
        };
    }
    Ok(())
}
