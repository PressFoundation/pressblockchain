use axum::{routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use rusqlite::{Connection, params};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;

#[derive(Serialize)]
struct OkResp { ok: bool }

#[derive(Deserialize)]
struct CreateProposal {
    title: String,
    kind: String,          // PARAM_CHANGE | UPGRADE | GRANT | COURT_POLICY
    key: Option<String>,   // variable key if applicable
    value: Option<String>, // proposed value
    fee_paid_tx: String,   // proof of fee payment (tx hash)
}

#[derive(Serialize)]
struct Proposal {
    id: i64,
    title: String,
    kind: String,
    key: Option<String>,
    value: Option<String>,
    fee_paid_tx: String,
    created_at: i64,
    status: String,        // OPEN | CLOSED | EXECUTED
}

async fn health() -> &'static str { "ok" }

async fn list(db: axum::extract::State<Arc<Mutex<Connection>>>) -> Json<Vec<Proposal>> {
    let db = db.lock().await;
    let mut stmt = db.prepare("SELECT id,title,kind,key,value,fee_paid_tx,created_at,status FROM proposals ORDER BY id DESC").unwrap();
    let rows = stmt.query_map([], |r| Ok(Proposal{
        id: r.get(0)?, title: r.get(1)?, kind: r.get(2)?,
        key: r.get(3)?, value: r.get(4)?,
        fee_paid_tx: r.get(5)?, created_at: r.get(6)?,
        status: r.get(7)?,
    })).unwrap();
    let mut out = vec![];
    for r in rows { out.push(r.unwrap()); }
    Json(out)
}

async fn create(db: axum::extract::State<Arc<Mutex<Connection>>>, Json(req): Json<CreateProposal>) -> Json<OkResp> {
    // NOTE: fee verification is enforced by on-chain + installer; API stores tx hash for explorers/indexer correlation.
    let ts = chrono_like_now();
    let db = db.lock().await;
    db.execute("INSERT INTO proposals(title,kind,key,value,fee_paid_tx,created_at,status) VALUES (?,?,?,?,?,?,?)",
        params![req.title, req.kind, req.key, req.value, req.fee_paid_tx, ts, "OPEN"]
    ).unwrap();
    Json(OkResp{ok:true})
}

// deterministic unix seconds without chrono dependency
fn chrono_like_now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_path = std::env::var("GOV_DB").unwrap_or_else(|_| "/state/governance.db".to_string());
    let mut conn = Connection::open(db_path)?;
    conn.execute("CREATE TABLE IF NOT EXISTS proposals(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL,
        kind TEXT NOT NULL,
        key TEXT,
        value TEXT,
        fee_paid_tx TEXT NOT NULL,
        created_at INTEGER NOT NULL,
        status TEXT NOT NULL
    )", [])?;

    let db = Arc::new(Mutex::new(conn));
    let addr: SocketAddr = "0.0.0.0:8801".parse().unwrap();
    let app = Router::new()
        .route("/health", get(health))
        .route("/proposals", get(list).post(create))
        .with_state(db);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
