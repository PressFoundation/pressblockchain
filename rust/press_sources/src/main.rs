use axum::{routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use rusqlite::{Connection, params};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

#[derive(Serialize)]
struct Health { ok: bool }

#[derive(Deserialize)]
struct RegisterReq { wallet: String, display_name: String, kind: String, website: Option<String> }

#[derive(Serialize)]
struct RegisterResp { ok: bool, id: i64 }

#[derive(Deserialize)]
struct RequestReq { article_id: String, requester_wallet: String, source_wallet: String, share_bps: i64, scope: String }

#[derive(Serialize)]
struct RequestResp { ok: bool, request_id: i64 }

#[derive(Deserialize)]
struct DecideReq { request_id: i64, source_wallet: String, decision: String }

#[derive(Serialize)]
struct DecideResp { ok: bool }

async fn health() -> Json<Health> { Json(Health{ok:true}) }

async fn cfg() -> Json<serde_json::Value> {
    let cfg = include_str!("../../../config/source_role.json");
    Json(serde_json::from_str(cfg).unwrap())
}

async fn list_sources(db: axum::extract::State<Arc<Mutex<Connection>>>) -> Json<Vec<serde_json::Value>> {
    let db = db.lock().await;
    let mut stmt = db.prepare("SELECT id,wallet,display_name,kind,website,created_at FROM sources ORDER BY id DESC").unwrap();
    let rows = stmt.query_map([], |r| {
        Ok(serde_json::json!({
            "id": r.get::<_,i64>(0)?,
            "wallet": r.get::<_,String>(1)?,
            "display_name": r.get::<_,String>(2)?,
            "kind": r.get::<_,String>(3)?,
            "website": r.get::<_,Option<String>>(4)?,
            "created_at": r.get::<_,i64>(5)?
        }))
    }).unwrap();
    Json(rows.map(|x| x.unwrap()).collect())
}

async fn register(db: axum::extract::State<Arc<Mutex<Connection>>>, Json(req): Json<RegisterReq>) -> Json<RegisterResp> {
    let now = now_unix();
    let mut db = db.lock().await;
    db.execute("INSERT INTO sources(wallet,display_name,kind,website,created_at) VALUES (?,?,?,?,?)",
        params![req.wallet, req.display_name, req.kind, req.website, now]).unwrap();
    let id = db.last_insert_rowid();
    Json(RegisterResp{ok:true, id})
}

async fn create_request(db: axum::extract::State<Arc<Mutex<Connection>>>, Json(req): Json<RequestReq>) -> Json<RequestResp> {
    let now = now_unix();
    // Production: enforce share caps, scope, and on-chain attestations.
    let mut db = db.lock().await;
    db.execute("INSERT INTO attach_requests(article_id,requester_wallet,source_wallet,share_bps,scope,status,created_at) VALUES (?,?,?,?,?,?,?)",
        params![req.article_id, req.requester_wallet, req.source_wallet, req.share_bps, req.scope, "pending", now]).unwrap();
    let id = db.last_insert_rowid();
    Json(RequestResp{ok:true, request_id:id})
}

async fn decide(db: axum::extract::State<Arc<Mutex<Connection>>>, Json(req): Json<DecideReq>) -> Json<DecideResp> {
    let mut db = db.lock().await;
    db.execute("UPDATE attach_requests SET status=?1 WHERE id=?2 AND source_wallet=?3",
        params![req.decision, req.request_id, req.source_wallet]).unwrap();
    Json(DecideResp{ok:true})
}

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = std::env::var("SOURCES_DB").unwrap_or_else(|_| "/state/sources.db".to_string());
    let mut conn = Connection::open(db_path)?;
    conn.execute("CREATE TABLE IF NOT EXISTS sources(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        wallet TEXT NOT NULL,
        display_name TEXT NOT NULL,
        kind TEXT NOT NULL,
        website TEXT,
        created_at INTEGER NOT NULL
    )", [])?;
    conn.execute("CREATE TABLE IF NOT EXISTS attach_requests(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        article_id TEXT NOT NULL,
        requester_wallet TEXT NOT NULL,
        source_wallet TEXT NOT NULL,
        share_bps INTEGER NOT NULL,
        scope TEXT NOT NULL,
        status TEXT NOT NULL,
        created_at INTEGER NOT NULL
    )", [])?;

    let db = Arc::new(Mutex::new(conn));
    let addr: SocketAddr = "0.0.0.0:8806".parse().unwrap();
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/v1/health", get(health))
        .route("/v1/config", get(cfg))
        .route("/v1/sources", get(list_sources))
        .route("/v1/register", post(register))
        .route("/v1/requests", post(create_request))
        .route("/v1/decide", post(decide))
        .with_state(db);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
