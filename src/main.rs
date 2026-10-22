mod agent;
mod bridge;
mod detection;
mod environments;
mod external;
mod model;
mod owned;
mod paths;
mod platform;
mod runtime;
mod server;
mod ssh;
mod store;
mod tools;
use model::*;
use serde_json::{json, Value};
use std::{fs, sync::Arc};
use tokio::io::AsyncWriteExt;

fn arg<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.windows(2)
        .find(|a| a[0] == name)
        .map(|a| a[1].as_str())
}
#[tokio::main]
async fn main() {
    if let Err(e) = run(std::env::args().skip(1).collect()).await {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
async fn run(args: Vec<String>) -> Result<()> {
    if let Ok(origin) = std::env::var("TETHER_ASKPASS_URL") {
        let token = std::env::var("TETHER_ASKPASS_TOKEN").map_err(|_| "缺少认证身份")?;
        let response=server::call(&origin,"/prompt",&token,json!({"groupId":std::env::var("TETHER_ASKPASS_GROUP").unwrap_or_default(),"text":args.join(" ")})).await?;
        println!("{}", response["answer"].as_str().ok_or("认证已取消")?);
        return Ok(());
    }
    #[cfg(unix)]
    if arg(&args, "--expected-uid").is_some_and(|uid| uid != unsafe { libc::geteuid() }.to_string())
    {
        return Err("WSL 默认用户已变化，请重新选择环境".into());
    }
    match args.first().map(String::as_str).unwrap_or("help") {
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
                json!({"protocol":1,"bridgeProtocol":bridge::PROTOCOL,"ssh":platform::decode(&out.stderr),"version":env!("CARGO_PKG_VERSION")})
            );
            Ok(())
        }
        "inspect-port" => {
            let v = serde_json::from_str(arg(&args, "--json").ok_or("缺少请求")?)
                .map_err(|e| e.to_string())?;
            println!("{}", detection::inspect(v).await?);
            Ok(())
        }
        "serve" => serve(paths::Paths::prepare(&args)?, &args).await,
        "instance" => {
            let paths = paths::Paths::prepare(&args)?;
            let descriptor: Value =
                serde_json::from_slice(&store::read(&paths.runtime.join("instance.json"))?)
                    .map_err(|e| e.to_string())?;
            println!("{descriptor}");
            Ok(())
        }
        "request" => {
            let paths = paths::Paths::prepare(&args)?;
            let desc: Value =
                serde_json::from_slice(&store::read(&paths.runtime.join("instance.json"))?)
                    .map_err(|e| e.to_string())?;
            let v = serde_json::from_str(arg(&args, "--json").ok_or("缺少 --json 请求")?)
                .map_err(|e| e.to_string())?;
            let out = server::call(
                desc["origin"].as_str().ok_or("无控制地址")?,
                "/api",
                desc["token"].as_str().ok_or("无实例身份")?,
                v,
            )
            .await?;
            println!("{out}");
            if out["ok"] == true {
                Ok(())
            } else {
                Err("动作失败".into())
            }
        }
        _ => {
            println!("Tether {}\n  serve [--data-dir ABSOLUTE] [--oriel-stdio]\n  request [--data-dir ABSOLUTE] --json JSON\n  check",env!("CARGO_PKG_VERSION"));
            Ok(())
        }
    }
}
async fn serve(paths: paths::Paths, args: &[String]) -> Result<()> {
    use fs2::FileExt;
    let bridge = args.iter().any(|a| a == "--bridge-stdio");
    let stdio = bridge || args.iter().any(|a| a == "--oriel-stdio");
    store::private_dir(&paths.runtime.join("runtime"))?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(paths.runtime.join("instance.lock"))
        .map_err(|e| e.to_string())?;
    if lock.try_lock_exclusive().is_err() {
        if stdio {
            return proxy(paths, bridge).await;
        }
        return Err("此配置已有后台实例，请使用 request 或原页面连接".into());
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let auth = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| e.to_string())?;
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let token = format!("{}{}", id(), id());
    let rt = runtime::Runtime::new(
        paths.config.clone(),
        paths.runtime.clone(),
        arg(args, "--environment")
            .unwrap_or(platform::local_id())
            .into(),
        arg(args, "--environment-label")
            .map(str::to_owned)
            .unwrap_or_else(platform::local_label),
        origin.clone(),
        token.clone(),
        format!("http://{}", auth.local_addr().unwrap()),
    );
    let hub = environments::Hub::new(rt.clone(), paths.data)?;
    let descriptor = json!({"protocol":1,"bridgeProtocol":bridge::PROTOCOL,"version":env!("CARGO_PKG_VERSION"),"environment":rt.environment,"instance":rt.instance,"origin":origin,"token":token,"pageUrl":format!("{origin}/#token={token}"),"ready":true});
    store::atomic(
        &paths.runtime.join("instance.json"),
        &serde_json::to_vec(&descriptor).unwrap(),
    )?;
    let output = Arc::new(tokio::sync::Mutex::new(tokio::io::stdout()));
    {
        let mut out = output.lock().await;
        out.write_all(format!("{descriptor}\n").as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        out.flush().await.map_err(|e| e.to_string())?;
    }
    tokio::spawn(server::auth(rt.clone(), auth));
    if stdio {
        let (mut requests, mut eof) = input_lines();
        let ended = hub.clone();
        tokio::spawn(async move {
            let _ = eof.changed().await;
            ended.exit().await;
        });
        let hub = hub.clone();
        let output = output.clone();
        tokio::spawn(async move {
            let mut tasks = tokio::task::JoinSet::new();
            while let Some(line) = requests.recv().await {
                if *hub.local.shutdown.borrow() {
                    break;
                }
                let parsed = serde_json::from_str::<Value>(&line);
                let (id, request) = match parsed {
                    Ok(v) if bridge => (Some(v["id"].clone()), v["request"].clone()),
                    Ok(v) => (None, v),
                    Err(e) => {
                        let _ = bridge::write(&output, None, Err(e.to_string())).await;
                        continue;
                    }
                };
                if bridge && !id.as_ref().is_some_and(|id| id.is_string()) {
                    break;
                }
                if bridge {
                    let h = hub.clone();
                    let o = output.clone();
                    tasks.spawn(async move {
                        let result = h.action(request).await;
                        let _ = bridge::write(&o, id, result).await;
                    });
                    while tasks.try_join_next().is_some() {}
                } else {
                    // Public stdio-v1 remains ordered; internal bridge requests are multiplexed.
                    let result = hub.action(request).await;
                    if bridge::write(&output, None, result).await.is_err() {
                        break;
                    }
                }
            }
            hub.exit().await;
            while tasks.join_next().await.is_some() {}
        });
    }
    let mut shutdown = rt.shutdown.subscribe();
    let stopped = hub.clone();
    axum::serve(listener, server::router(hub))
        .with_graceful_shutdown(async move {
            tokio::select! {_=tokio::signal::ctrl_c()=>{},_=shutdown.changed()=>{}}
            stopped.exit().await;
        })
        .await
        .map_err(|e| e.to_string())?;
    let _ = fs::remove_file(paths.runtime.join("instance.json"));
    drop(lock);
    Ok(())
}
fn input_lines() -> (
    tokio::sync::mpsc::Receiver<String>,
    tokio::sync::watch::Receiver<bool>,
) {
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    let (eof, ended) = tokio::sync::watch::channel(false);
    std::thread::spawn(move || {
        use std::io::{BufRead, Read};
        let input = std::io::stdin();
        let mut reader = std::io::BufReader::new(input.lock());
        loop {
            let mut line = String::new();
            match reader
                .by_ref()
                .take(4 * 1024 * 1024 + 1)
                .read_line(&mut line)
            {
                Ok(0) | Err(_) => break,
                Ok(_) if line.len() > 4 * 1024 * 1024 => break,
                Ok(_) => {
                    if tx.blocking_send(line).is_err() {
                        break;
                    }
                }
            }
        }
        eof.send_replace(true);
    });
    (rx, ended)
}
async fn proxy(paths: paths::Paths, bridge: bool) -> Result<()> {
    let descriptor: Value =
        serde_json::from_slice(&store::read(&paths.runtime.join("instance.json"))?)
            .map_err(|e| e.to_string())?;
    if descriptor["version"] != env!("CARGO_PKG_VERSION") {
        return Err("已有后台版本不同，请正常退出后重试".into());
    }
    let origin = descriptor["origin"]
        .as_str()
        .ok_or("实例无控制地址")?
        .to_owned();
    let token = descriptor["token"]
        .as_str()
        .ok_or("实例无调用身份")?
        .to_owned();
    let status = server::call(&origin, "/api", &token, json!({"action":"status"})).await?;
    if status["ok"] != true || status["value"]["instance"] != descriptor["instance"] {
        return Err("已有实例身份校验失败".into());
    }
    println!("{descriptor}");
    let (mut input, mut eof) = input_lines();
    let o = origin.clone();
    let t = token.clone();
    tokio::spawn(async move {
        let _ = eof.changed().await;
        let _ = server::call(&o, "/api", &t, json!({"action":"shutdown"})).await;
    });
    let output = Arc::new(tokio::sync::Mutex::new(tokio::io::stdout()));
    let mut tasks = tokio::task::JoinSet::new();
    while let Some(line) = input.recv().await {
        let v: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        let id = bridge.then(|| v["id"].clone());
        let request = if bridge { v["request"].clone() } else { v };
        let origin = origin.clone();
        let token = token.clone();
        let output = output.clone();
        let task = async move {
            let result = server::call(&origin, "/api", &token, request)
                .await
                .and_then(|v| {
                    if v["ok"] == true {
                        Ok(v["value"].clone())
                    } else {
                        Err(v["error"].as_str().unwrap_or("动作失败").into())
                    }
                });
            let _ = bridge::write(&output, id, result).await;
        };
        if bridge {
            tasks.spawn(task);
        } else {
            task.await;
        }
    }
    let _ = server::call(&origin, "/api", &token, json!({"action":"shutdown"})).await;
    while tasks.join_next().await.is_some() {}
    Ok(())
}
