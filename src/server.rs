use crate::{model::*, runtime::Runtime};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
fn authorized(rt: &Runtime, headers: &HeaderMap) -> bool {
    let authority = rt.origin.trim_start_matches("http://");
    headers.get("host").and_then(|v| v.to_str().ok()) == Some(authority)
        && headers
            .get("origin")
            .is_none_or(|v| v.to_str().ok() == Some(rt.origin.as_str()))
        && headers.get("authorization").and_then(|v| v.to_str().ok())
            == Some(format!("Bearer {}", rt.token).as_str())
}
async fn api(State(rt): State<Arc<Runtime>>, headers: HeaderMap, Json(v): Json<Value>) -> Response {
    if !authorized(&rt, &headers) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"error":"调用身份或来源无效"})),
        )
            .into_response();
    }
    match rt.action(v).await {
        Ok(v) => Json(json!({"ok":true,"value":v})).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"ok":false,"error":e}))).into_response(),
    }
}
async fn assets(State(rt): State<Arc<Runtime>>, req: Request) -> Response {
    if req.headers().get("host").and_then(|h| h.to_str().ok())
        != Some(rt.origin.trim_start_matches("http://"))
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let raw = req.uri().path();
    let path = if raw == "/" {
        "index.html"
    } else {
        raw.trim_start_matches('/')
    };
    if path.split('/').any(|s| s == ".." || s.starts_with('.')) || path.contains(['%', '\\']) {
        return StatusCode::NOT_FOUND.into_response();
    }
    let root = std::env::var_os("TETHER_UI_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .join("dist")
        });
    match tokio::fs::read(root.join(path)).await {
        Ok(bytes) => {
            let mime = if path.ends_with(".js") {
                "text/javascript"
            } else if path.ends_with(".css") {
                "text/css"
            } else if path.ends_with(".svg") {
                "image/svg+xml"
            } else {
                "text/html; charset=utf-8"
            };
            ([( "Content-Type",mime),("Content-Security-Policy","default-src 'self'; connect-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'"),("Cache-Control","no-store"),("Referrer-Policy","no-referrer"),("X-Content-Type-Options","nosniff")],bytes).into_response()
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            "界面资源未构建或未放在程序旁的 dist 目录。请执行 npm run build。",
        )
            .into_response(),
    }
}
pub fn router(rt: Arc<Runtime>) -> Router {
    Router::new()
        .route("/api", post(api))
        .fallback(get(assets))
        .layer(axum::extract::DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(rt)
}
// A different port and credential are used by askpass. The UI cannot manufacture prompts.
pub async fn auth(rt: Arc<Runtime>, listener: tokio::net::TcpListener) {
    let app=Router::new().route("/prompt",post(|State(rt):State<Arc<Runtime>>,headers:HeaderMap,Json(v):Json<Value>|async move{if headers.contains_key("origin")||headers.get("authorization").and_then(|s|s.to_str().ok())!=Some(format!("Bearer {}",rt.auth_token).as_str()){return (StatusCode::FORBIDDEN,Json(json!({"error":"unauthorized"})));}let result=rt.prompt(v["groupId"].as_str().unwrap_or(""),v["text"].as_str().unwrap_or("").into()).await;match result{Ok(answer)=>(StatusCode::OK,Json(json!({"answer":answer}))),Err(e)=>(StatusCode::BAD_REQUEST,Json(json!({"error":e})))}})).layer(axum::extract::DefaultBodyLimit::max(16384)).with_state(rt.clone());
    let mut shutdown = rt.shutdown.subscribe();
    let _ = axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown.changed().await;
        })
        .await;
}
pub fn envelope(v: std::result::Result<Value, String>) -> Value {
    match v {
        Ok(v) => json!({"ok":true,"value":v}),
        Err(e) => json!({"ok":false,"error":e}),
    }
}
// Small loopback-only HTTP client shared by CLI and askpass; no global HTTP proxy.
pub async fn call(origin: &str, path: &str, token: &str, value: Value) -> Result<Value> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let addr = origin
        .strip_prefix("http://127.0.0.1:")
        .ok_or("控制地址须为 IPv4 回环")?;
    let port = addr.parse::<u16>().map_err(|_| "控制端口无效")?;
    let mut stream = tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec(&value).unwrap();
    let head=format!("POST {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len());
    stream
        .write_all(head.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    stream.write_all(&bytes).await.map_err(|e| e.to_string())?;
    let mut response = vec![];
    tokio::time::timeout(
        std::time::Duration::from_secs(130),
        stream.take(4 * 1024 * 1024).read_to_end(&mut response),
    )
    .await
    .map_err(|_| "动作超时")?
    .map_err(|e| e.to_string())?;
    let split = response
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("HTTP 返回无效")?;
    serde_json::from_slice(&response[split + 4..]).map_err(|e| format!("返回格式错误：{e}"))
}
