use axum::{routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use rusqlite::{Connection, params};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

#[derive(Serialize)]
struct Health { ok: bool }

#[derive(Deserialize)]
struct ClaimReq { wallet: String, bounty_key: String, proof_ref: Option<String> }

#[derive(Serialize)]
struct ClaimResp { ok: bool, message: String }

async fn health() -> Json<Health> { Json(Health{ok:true}) }

async fn pools() -> Json<serde_json::Value> {
    let cfg = include_str!("../../../config/distribution_pools.json");
    Json(serde_json::from_str(cfg).unwrap())
}

async fn bounties() -> Json<serde_json::Value> {
    let cfg = include_str!("../../../config/bounties.json");
    Json(serde_json::from_str(cfg).unwrap())
}

async fn claim(db: axum::extract::State<Arc<Mutex<Connection>>>, Json(req): Json<ClaimReq>) -> Json<ClaimResp> {
    let now = now_unix();
    let mut db = db.lock().await;

    let mut stmt = db.prepare("SELECT last_claimed_at FROM claims WHERE wallet=?1 AND bounty_key=?2").unwrap();
    let mut rows = stmt.query(params![req.wallet, req.bounty_key]).unwrap();
    if let Some(row) = rows.next().unwrap() {
        let last: i64 = row.get(0).unwrap();
        if now - last < 3600 {
            return Json(ClaimResp{ok:false, message:"Cooldown active".to_string()});
        }
        db.execute("UPDATE claims SET last_claimed_at=?1, proof_ref=?2 WHERE wallet=?3 AND bounty_key=?4",
            params![now, req.proof_ref, req.wallet, req.bounty_key]).unwrap();
        return Json(ClaimResp{ok:true, message:"Claim recorded".to_string()});
    }

    db.execute("INSERT INTO claims(wallet,bounty_key,last_claimed_at,proof_ref) VALUES (?,?,?,?)",
        params![req.wallet, req.bounty_key, now, req.proof_ref]).unwrap();
    Json(ClaimResp{ok:true, message:"Claim recorded".to_string()})
}

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = std::env::var("REWARDS_DB").unwrap_or_else(|_| "/state/rewards.db".to_string());
    let mut conn = Connection::open(db_path)?;
    conn.execute("CREATE TABLE IF NOT EXISTS claims(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        wallet TEXT NOT NULL,
        bounty_key TEXT NOT NULL,
        last_claimed_at INTEGER NOT NULL,
        proof_ref TEXT
    )", [])?;

    let db = Arc::new(Mutex::new(conn));
    let addr: SocketAddr = "0.0.0.0:8805".parse().unwrap();
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/v1/health", get(health))
        .route("/v1/pools", get(pools))
        .route("/v1/bounties", get(bounties))
        .route("/v1/claim", post(claim))
        .with_state(db);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
