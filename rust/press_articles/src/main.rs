use axum::{routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use rusqlite::{Connection, params};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Serialize)]
struct Health { ok: bool }

#[derive(Deserialize)]
struct SubmitReq {
    outlet_id: String,
    author_wallet: String,
    title: String,
    content_hash: String,
    canonical_url: Option<String>,
    arweave_import: Option<bool>,
}

#[derive(Serialize)]
struct SubmitResp { ok: bool, article_id: String, state: String }

#[derive(Deserialize)]
struct VoteReq {
    article_id: String,
    voter_wallet: String,
    role: String, // reader/journalist/editor/source
    direction: String, // up/down
    fee_paid_press: f64,
}

#[derive(Serialize)]
struct VoteResp { ok: bool, state: String, totals: Totals }

#[derive(Serialize, Clone)]
struct Totals { up: i64, down: i64, unique_voters: i64 }

#[derive(Serialize)]
struct ArticleStatus { article_id: String, state: String, totals: Totals, ends_at: i64 }

async fn health() -> Json<Health> { Json(Health{ok:true}) }

async fn config() -> Json<serde_json::Value> {
    Json(serde_json::from_str(include_str!("../../../config/article_lifecycle.json")).unwrap())
}

fn now_unix() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

fn totals(db:&Connection, article_id:&str) -> Totals {
    let up:i64 = db.query_row("SELECT COUNT(*) FROM votes WHERE article_id=?1 AND direction='up'",
        params![article_id], |r| r.get(0)).unwrap_or(0);
    let down:i64 = db.query_row("SELECT COUNT(*) FROM votes WHERE article_id=?1 AND direction='down'",
        params![article_id], |r| r.get(0)).unwrap_or(0);
    let uniq:i64 = db.query_row("SELECT COUNT(DISTINCT voter_wallet) FROM votes WHERE article_id=?1",
        params![article_id], |r| r.get(0)).unwrap_or(0);
    Totals{up,down,unique_voters:uniq}
}

async fn submit(db: axum::extract::State<Arc<Mutex<Connection>>>, Json(req): Json<SubmitReq>) -> Json<SubmitResp> {
    // Production: verify PRESS fee payment on-chain before accepting; emit on-chain event; kick off vote window.
    let article_id = Uuid::new_v4().to_string();
    let now = now_unix();
    let ends_at = now + 72*3600;
    let mut db = db.lock().await;
    db.execute("INSERT INTO articles(article_id,outlet_id,author_wallet,title,content_hash,canonical_url,state,created_at,ends_at,arweave_import) VALUES (?,?,?,?,?,?,?,?,?,?)",
        params![article_id, req.outlet_id, req.author_wallet, req.title, req.content_hash, req.canonical_url, "voting", now, ends_at, req.arweave_import.unwrap_or(false)]).unwrap();
    Json(SubmitResp{ok:true, article_id, state:"voting".into()})
}

async fn vote(db: axum::extract::State<Arc<Mutex<Connection>>>, Json(req): Json<VoteReq>) -> Json<VoteResp> {
    // Production: enforce per-role vote fee + rate limits; verify tx; update on-chain tally.
    let now = now_unix();
    let mut db = db.lock().await;

    // Prevent double voting per wallet per article
    let exists:i64 = db.query_row("SELECT COUNT(*) FROM votes WHERE article_id=?1 AND voter_wallet=?2",
        params![req.article_id, req.voter_wallet], |r| r.get(0)).unwrap_or(0);
    if exists > 0 {
        let t = totals(&db, &req.article_id);
        return Json(VoteResp{ok:false, state:"voting".into(), totals:t});
    }

    db.execute("INSERT INTO votes(article_id,voter_wallet,role,direction,fee_paid_press,created_at) VALUES (?,?,?,?,?,?)",
        params![req.article_id, req.voter_wallet, req.role, req.direction, req.fee_paid_press, now]).unwrap();

    // Auto-close after ends_at
    let ends_at:i64 = db.query_row("SELECT ends_at FROM articles WHERE article_id=?1", params![req.article_id], |r| r.get(0)).unwrap_or(0);
    let mut state:String = db.query_row("SELECT state FROM articles WHERE article_id=?1", params![req.article_id], |r| r.get(0)).unwrap_or("voting".into());
    if now >= ends_at && state == "voting" {
        // Production: evaluate thresholds/quorum and set approved/rejected + emit event
        state = "approved".into();
        db.execute("UPDATE articles SET state=?1 WHERE article_id=?2", params![state, req.article_id]).ok();
    }

    let t = totals(&db, &req.article_id);
    Json(VoteResp{ok:true, state, totals:t})
}

async fn status(db: axum::extract::State<Arc<Mutex<Connection>>>, axum::extract::Path(article_id): axum::extract::Path<String>) -> Json<ArticleStatus> {
    let db = db.lock().await;
    let state:String = db.query_row("SELECT state FROM articles WHERE article_id=?1", params![article_id], |r| r.get(0)).unwrap_or("unknown".into());
    let ends_at:i64 = db.query_row("SELECT ends_at FROM articles WHERE article_id=?1", params![article_id], |r| r.get(0)).unwrap_or(0);
    let t = totals(&db, &article_id);
    Json(ArticleStatus{article_id, state, totals:t, ends_at})
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = std::env::var("ARTICLES_DB").unwrap_or_else(|_| "/state/articles.db".to_string());
    let mut conn = Connection::open(db_path)?;
    conn.execute("CREATE TABLE IF NOT EXISTS articles(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        article_id TEXT NOT NULL UNIQUE,
        outlet_id TEXT NOT NULL,
        author_wallet TEXT NOT NULL,
        title TEXT NOT NULL,
        content_hash TEXT NOT NULL,
        canonical_url TEXT,
        state TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        ends_at INTEGER NOT NULL,
        arweave_import INTEGER NOT NULL
    )", [])?;
    conn.execute("CREATE TABLE IF NOT EXISTS votes(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        article_id TEXT NOT NULL,
        voter_wallet TEXT NOT NULL,
        role TEXT NOT NULL,
        direction TEXT NOT NULL,
        fee_paid_press REAL NOT NULL,
        created_at INTEGER NOT NULL
    )", [])?;

    let db = Arc::new(Mutex::new(conn));
    let addr: SocketAddr = "0.0.0.0:8808".parse().unwrap();
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/v1/health", get(health))
        .route("/v1/config", get(config))
        .route("/v1/articles/submit", post(submit))
        .route("/v1/articles/vote", post(vote))
        .route("/v1/articles/:id", get(status))
        .with_state(db);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
