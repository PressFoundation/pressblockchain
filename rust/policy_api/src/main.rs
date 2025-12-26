\
use axum::{routing::{get, post}, Json, Router, extract::State, http::{HeaderMap, StatusCode}};
use serde::{Deserialize, Serialize};
use tower_http::cors::{CorsLayer, Any};
use tracing_subscriber::EnvFilter;
use std::{path::PathBuf, net::SocketAddr};

#[derive(Clone)]
struct AppState {
    rbac_path: PathBuf,
    auth_up: String,
}

#[derive(Serialize)]
struct Health { ok: bool }

#[derive(Deserialize)]
struct AllowReq {
    action: String,
}

#[derive(Serialize)]
struct AllowResp {
    action: String,
    allowed: bool,
    reason: String,
    roles: Vec<String>,
}

#[derive(Deserialize)]
struct RbacFile {
    actions: serde_json::Map<String, serde_json::Value>,
}

fn roles_any(v: &serde_json::Value) -> Vec<String> {
    v.get("rolesAny").and_then(|x| x.as_array())
        .map(|a| a.iter().filter_map(|i| i.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_else(|| vec![])
}

fn login_required(v: &serde_json::Value) -> bool {
    v.get("loginRequired").and_then(|x| x.as_bool()).unwrap_or(false)
}

async fn read_rbac(path: &PathBuf) -> Result<RbacFile, ()> {
    let s = tokio::fs::read_to_string(path).await.map_err(|_| ())?;
    serde_json::from_str(&s).map_err(|_| ())
}

async fn auth_me(auth_up: &str, headers: &HeaderMap) -> Option<serde_json::Value> {
    let auth = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
    if !auth.starts_with("Bearer ") { return None; }
    let url = format!("{}/me", auth_up.trim_end_matches('/'));
    reqwest::Client::new()
        .get(url)
        .header("authorization", auth)
        .send().await.ok()?
        .json().await.ok()
}

async fn health() -> Json<Health> { Json(Health{ ok:true }) }

async fn allow(State(st): State<AppState>, headers: HeaderMap, Json(req): Json<AllowReq>) -> Result<Json<AllowResp>, (StatusCode, String)> {
    let rbac = read_rbac(&st.rbac_path).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "RBAC_NOT_FOUND".into()))?;
    let rule = rbac.actions.get(&req.action).ok_or((StatusCode::NOT_FOUND, "UNKNOWN_ACTION".into()))?;

    let lr = login_required(rule);
    let mut roles: Vec<String> = vec![];

    if lr {
        let me = auth_me(&st.auth_up, &headers).await.ok_or((StatusCode::UNAUTHORIZED, "LOGIN_REQUIRED".into()))?;
        roles = me.get("roles")
            .and_then(|x| x.as_array())
            .map(|a| a.iter().filter_map(|i| i.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_else(|| vec![]);
        // implicit reader for authenticated wallet
        roles.push("reader".into());
    }

    let allowed_roles = roles_any(rule);
    let allowed = if !lr {
        true
    } else if allowed_roles.is_empty() {
        true
    } else {
        roles.iter().any(|r| allowed_roles.contains(r))
    };

    let reason = if !lr { "PUBLIC".into() }
    else if allowed { "ALLOWED".into() }
    else { "INSUFFICIENT_ROLE".into() };

    Ok(Json(AllowResp{ action: req.action, allowed, reason, roles }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let rbac_path = std::env::var("PRESS_RBAC_PATH").unwrap_or_else(|_| "/config/rbac.json".to_string());
    let auth_up = std::env::var("AUTH_UPSTREAM").unwrap_or_else(|_| "http://auth-api:8788".to_string());

    let st = AppState { rbac_path: rbac_path.into(), auth_up };

    let app = Router::new()
        .route("/health", get(health))
        .route("/allow", post(allow))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(st);

    let addr: SocketAddr = "0.0.0.0:8790".parse().unwrap();
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app).await.unwrap();
}
