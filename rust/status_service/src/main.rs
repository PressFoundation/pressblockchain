\
use axum::{routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::{env, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    cfg: Arc<RwLock<StatusConfig>>,
    state_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusTarget {
    id: String,
    name: String,
    url: String,
    kind: String, // "http"
    critical: bool,
    json_field: Option<String>,
    json_bool_required: Option<bool>,
    json_num_field: Option<String>,
    json_num_max: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StatusConfig {
    brand: String,
    public_base_url: String,
    targets: Vec<StatusTarget>,
    refresh_seconds: u64,
}

impl Default for StatusConfig {
    fn default() -> Self {
        Self {
            brand: "Press Status".into(),
            public_base_url: "https://status.pressblockchain.io".into(),
            targets: vec![
                                StatusTarget{ id:"gateway".into(), name:"Gateway".into(), url:"http://deployer-gateway:8085/health".into(), kind:"http".into(), critical:true, json_field: None, json_bool_required: None , json_num_field: None, json_num_max: None},
                StatusTarget{ id:"rpc".into(), name:"RPC".into(), url:"http://press-rpc:8545".into(), kind:"http".into(), critical:true, json_field: None, json_bool_required: None , json_num_field: None, json_num_max: None},
                StatusTarget{ id:"indexer".into(), name:"Indexer".into(), url:"http://press-indexer:8786/health".into(), kind:"http".into(), critical:true, json_field: None, json_bool_required: None , json_num_field: None, json_num_max: None},
                StatusTarget{ id:"query".into(), name:"Query API".into(), url:"http://query-api:8787/health".into(), kind:"http".into(), critical:true, json_field: None, json_bool_required: None , json_num_field: None, json_num_max: None},
                StatusTarget{ id:"bots_discord".into(), name:"Bots — Discord".into(), url:"http://press-bots:8790/health".into(), kind:"json".into(), critical:false, json_field: Some("discord_connected".into()), json_bool_required: Some(true) , json_num_field: None, json_num_max: None},
                StatusTarget{ id:"bots_telegram".into(), name:"Bots — Telegram".into(), url:"http://press-bots:8790/health".into(), kind:"json".into(), critical:false, json_field: Some("telegram_connected".into()), json_bool_required: Some(true) , json_num_field: None, json_num_max: None},
                StatusTarget{ id:"bots_heartbeat".into(), name:"Bots — On-chain Heartbeat".into(), url:"http://press-indexer:8786/heartbeats/latest?service=press-bots".into(), kind:"json".into(), critical:false, json_field: Some("ok".into()), json_bool_required: Some(true), json_num_field: Some("age_sec".into()), json_num_max: Some(600) },
                StatusTarget{ id:"exchange".into(), name:"Exchange UI".into(), url:"http://exchange-ui:8080/".into(), kind:"http".into(), critical:false, json_field: None, json_bool_required: None , json_num_field: None, json_num_max: None},
            ],
            refresh_seconds: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct TargetResult {
    id: String,
    name: String,
    url: String,
    ok: bool,
    status: u16,
    latency_ms: u128,
    details: Option<serde_json::Value>,
    checked_at: i64,
    critical: bool,
    json_field: Option<String>,
    json_bool_required: Option<bool>,
    json_num_field: Option<String>,
    json_num_max: Option<i64>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct Summary {
    ok: bool,
    brand: String,
    checked_at: i64,
    healthy: usize,
    total: usize,
    critical_down: usize,
    results: Vec<TargetResult>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let state_path = PathBuf::from(env::var("PRESS_STATUS_STATE").unwrap_or_else(|_| "/state/status_config.json".into()));
    let cfg = load_or_default(&state_path).await;

    let state = AppState {
        cfg: Arc::new(RwLock::new(cfg)),
        state_path,
    };

    let app = Router::new()
        .route("/health", get(|| async { Json(serde_json::json!({"ok":true,"service":"press_status_service"})) }))
        .route("/api/status/config", get(get_config).post(set_config))
        .route("/api/status/summary", get(get_summary))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let bind = env::var("BIND").unwrap_or_else(|_| "0.0.0.0:8791".into());
    let addr: SocketAddr = bind.parse()?;
    info!("press_status_service listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

async fn load_or_default(path: &PathBuf) -> StatusConfig {
    if let Ok(bytes) = tokio::fs::read(path).await {
        if let Ok(cfg) = serde_json::from_slice::<StatusConfig>(&bytes) {
            return cfg;
        }
    }
    let mut cfg = StatusConfig::default();
    if let Ok(v) = env::var("PRESS_STATUS_BRAND") { if !v.is_empty() { cfg.brand = v; } }
    if let Ok(v) = env::var("PRESS_STATUS_PUBLIC_URL") { if !v.is_empty() { cfg.public_base_url = v; } }
    cfg
}

async fn persist(state: &AppState) -> anyhow::Result<()> {
    let cfg = state.cfg.read().await;
    let bytes = serde_json::to_vec_pretty(&*cfg)?;
    if let Some(parent) = state.state_path.parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::write(&state.state_path, bytes).await?;
    Ok(())
}

async fn get_config(axum::extract::State(state): axum::extract::State<AppState>) -> Json<serde_json::Value> {
    let cfg = state.cfg.read().await;
    Json(serde_json::json!({"ok": true, "config": cfg.clone()}))
}

async fn set_config(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<StatusConfig>,
) -> Json<serde_json::Value> {
    {
        let mut cfg = state.cfg.write().await;
        *cfg = payload;
    }
    if let Err(e) = persist(&state).await {
        return Json(serde_json::json!({"ok": false, "error": e.to_string()}));
    }
    Json(serde_json::json!({"ok": true}))
}

async fn get_summary(axum::extract::State(state): axum::extract::State<AppState>) -> Json<Summary> {
    let cfg = state.cfg.read().await.clone();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1400))
        .build()
        .expect("client");

    let mut results = Vec::new();
    for t in cfg.targets.iter() {
        let start = std::time::Instant::now();
        let mut ok = false;
        let mut code = 0u16;
        let mut err: Option<String> = None;
        let mut details: Option<serde_json::Value> = None;

        // HTTP health by GET; if url is a raw RPC base, it should still respond with something
        match client.get(&t.url).send().await {
            Ok(resp) => {
                code = resp.status().as_u16();
                // For JSON targets, parse and enforce booleans
                let body = resp.text().await.unwrap_or_default();
                if t.kind == "json" {
                    match serde_json::from_str::<serde_json::Value>(&body) {
                        Ok(v) => {
                            details = Some(v.clone());
                            ok = (code >= 200 && code < 400);

                            // boolean enforcement
                            if let Some(field) = &t.json_field {
                                if let Some(reqb) = t.json_bool_required {
                                    let val = v.get(field).and_then(|x| x.as_bool()).unwrap_or(false);
                                    ok = ok && (val == reqb);
                                }
                            }

                            // numeric max enforcement (e.g., age_sec <= 600)
                            if let Some(nf) = &t.json_num_field {
                                if let Some(maxv) = t.json_num_max {
                                    let val = v.get(nf).and_then(|x| x.as_i64()).unwrap_or(i64::MAX);
                                    ok = ok && (val <= maxv);
                                }
                            }
                        }
                        Err(_) => {
                            ok = false;
                            err = Some("invalid_json".into());
                        }
                    }
                } else {
                    ok = resp.status().is_success() || resp.status().as_u16() == 405; // RPC base may 405 on GET
                }
            }
            Err(e) => {
                err = Some(e.to_string());
            }
        }
        let latency = start.elapsed().as_millis();
        let checked_at = time::OffsetDateTime::now_utc().unix_timestamp();

        results.push(TargetResult{
            id: t.id.clone(),
            name: t.name.clone(),
            url: t.url.clone(),
            ok,
            status: code,
            latency_ms: latency,
            checked_at,
            critical: t.critical,
            error: err,
            details,
        });
    }

    let total = results.len();
    let healthy = results.iter().filter(|r| r.ok).count();
    let critical_down = results.iter().filter(|r| r.critical && !r.ok).count();

    Json(Summary{
        ok: critical_down == 0,
        brand: cfg.brand,
        checked_at: time::OffsetDateTime::now_utc().unix_timestamp(),
        healthy,
        total,
        critical_down,
        results,
    })
}