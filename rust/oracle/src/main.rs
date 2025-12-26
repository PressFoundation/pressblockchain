\
use axum::{routing::{get, post}, Json, Router, extract::{State}};
use serde::{Deserialize, Serialize};
use tower_http::cors::{CorsLayer, Any};
use tracing_subscriber::EnvFilter;
use sqlx::{SqlitePool, Row};
use sha2::{Sha256, Digest};
use std::collections::HashSet;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
}

#[derive(Serialize)]
struct Health { ok: bool, service: &'static str }

#[derive(Deserialize)]
struct AnalyzeReq {
    url: Option<String>,
    title: Option<String>,
    content: String,
    outlet: Option<String>,
    author_wallet: Option<String>,
}

#[derive(Serialize)]
struct SimilarMatch {
    report_id: i64,
    similarity: f32,
}

#[derive(Serialize)]
struct AnalyzeResp {
    content_hash: String,
    similarity_score: f32,
    closest_match: Option<SimilarMatch>,
    copyright_risk: String,
    conflict_flags: Vec<String>,
    suggested_tags: Vec<String>,
    oracle_report_id: i64,
}

/// RR25 Oracle upgrades:
/// - similarity heuristic (Jaccard on token sets) against recent reports
/// - copyright risk derived from similarity score
///
/// Future passes will add crawling, embeddings, contradiction sources, and on-chain anchoring.
async fn health() -> Json<Health> { Json(Health{ ok:true, service:"press_oracle" }) }

fn tokenize(s: &str) -> HashSet<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4)
        .take(4000)
        .map(|w| w.to_string())
        .collect()
}

fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() || b.is_empty() { return 0.0; }
    let inter = a.intersection(b).count() as f32;
    let uni = a.union(b).count() as f32;
    if uni <= 0.0 { 0.0 } else { inter / uni }
}

async fn analyze(State(st): State<AppState>, Json(req): Json<AnalyzeReq>) -> Json<AnalyzeResp> {
    // Deterministic content hash
    let mut h = Sha256::new();
    h.update(req.content.as_bytes());
    let hash = hex::encode(h.finalize());

    let tokens = tokenize(&req.content);
    let tokens_json = serde_json::to_string(&tokens.iter().take(800).collect::<Vec<_>>()).unwrap_or("[]".into());

    // Compare against recent reports
    let mut best: Option<(i64, f32)> = None;
    let rows = sqlx::query("SELECT id, tokens_json FROM oracle_reports ORDER BY id DESC LIMIT 200")
        .fetch_all(&st.db).await.unwrap_or_default();
    for r in rows {
        let id: i64 = r.get("id");
        let tj: String = r.get("tokens_json");
        let prev_vec: Vec<String> = serde_json::from_str(&tj).unwrap_or_default();
        let prev: HashSet<String> = prev_vec.into_iter().collect();
        let sim = jaccard(&tokens, &prev);
        if best.map(|b| sim > b.1).unwrap_or(true) {
            best = Some((id, sim));
        }
    }

    let similarity_score = best.map(|b| b.1).unwrap_or(0.0);

    let copyright_risk = if similarity_score >= 0.82 { "high" }
        else if similarity_score >= 0.62 { "medium" }
        else { "low" }.to_string();

    let conflict_flags: Vec<String> = vec![]; // RR26 will add contradiction sources
    let suggested_tags = vec!["press".to_string(), "news".to_string(), "oracle".to_string()];

    let now = time::OffsetDateTime::now_utc().to_string();
    let excerpt = req.content.chars().take(520).collect::<String>();

    let rowid = sqlx::query(
        "INSERT INTO oracle_reports (content_hash, url, title, outlet, author_wallet, similarity_score, copyright_risk, conflict_flags_json, tags_json, content_excerpt, tokens_json, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&hash)
    .bind(req.url.clone().unwrap_or_default())
    .bind(req.title.clone().unwrap_or_default())
    .bind(req.outlet.clone().unwrap_or_default())
    .bind(req.author_wallet.clone().unwrap_or_default())
    .bind(similarity_score)
    .bind(&copyright_risk)
    .bind(serde_json::to_string(&conflict_flags).unwrap())
    .bind(serde_json::to_string(&suggested_tags).unwrap())
    .bind(&excerpt)
    .bind(&tokens_json)
    .bind(&now)
    .execute(&st.db)
    .await
    .ok()
    .map(|r| r.last_insert_rowid())
    .unwrap_or(0);

    Json(AnalyzeResp{
        content_hash: hash,
        similarity_score,
        closest_match: best.map(|(id, sim)| SimilarMatch{ report_id: id, similarity: sim }),
        copyright_risk,
        conflict_flags,
        suggested_tags,
        oracle_report_id: rowid,
    })
}

#[derive(Serialize)]
struct OracleReport {
    id: i64,
    content_hash: String,
    url: String,
    title: String,
    outlet: String,
    author_wallet: String,
    similarity_score: f32,
    copyright_risk: String,
    conflict_flags_json: String,
    tags_json: String,
    content_excerpt: String,
    created_at: String,
}

async fn latest(State(st): State<AppState>) -> Json<Vec<OracleReport>> {
    let rows = sqlx::query("SELECT id, content_hash, url, title, outlet, author_wallet, similarity_score, copyright_risk, conflict_flags_json, tags_json, content_excerpt, created_at FROM oracle_reports ORDER BY id DESC LIMIT 100")
        .fetch_all(&st.db).await.unwrap_or_default();
    let out = rows.into_iter().map(|r| OracleReport{
        id: r.get("id"),
        content_hash: r.get("content_hash"),
        url: r.get("url"),
        title: r.get("title"),
        outlet: r.get("outlet"),
        author_wallet: r.get("author_wallet"),
        similarity_score: r.get::<f64,_>("similarity_score") as f32,
        copyright_risk: r.get("copyright_risk"),
        conflict_flags_json: r.get("conflict_flags_json"),
        tags_json: r.get("tags_json"),
        content_excerpt: r.get("content_excerpt"),
        created_at: r.get("created_at"),
    }).collect();
    Json(out)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let db_url = std::env::var("ORACLE_DB").unwrap_or_else(|_| "sqlite:/data/oracle.db".into());
    let db = SqlitePool::connect(&db_url).await.expect("db");

    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS oracle_reports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content_hash TEXT NOT NULL,
            url TEXT NOT NULL,
            title TEXT NOT NULL,
            outlet TEXT NOT NULL,
            author_wallet TEXT NOT NULL,
            similarity_score REAL NOT NULL,
            copyright_risk TEXT NOT NULL,
            conflict_flags_json TEXT NOT NULL,
            tags_json TEXT NOT NULL,
            content_excerpt TEXT NOT NULL,
            tokens_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
    "#).execute(&db).await.ok();

    let st = AppState{ db };

    let app = Router::new()
        .route("/health", get(health))
        .route("/analyze", post(analyze))
        .route("/reports/latest", get(latest))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(st);

    let addr = "0.0.0.0:8796".parse().unwrap();
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app).await.unwrap();
}
