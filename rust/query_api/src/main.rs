use axum::{routing::get, Json, Router, extract::State};
use serde::Serialize;
use sqlx::PgPool;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
struct AppState {
    db: PgPool,
}

#[derive(Serialize)]
struct Health { ok: bool }

async fn health() -> Json<Health> { Json(Health { ok: true }) }

#[derive(Serialize)]
struct ContractRow {
    name: String,
    address: String,
    chain_id: i64,
    deployed_at: chrono::DateTime<chrono::Utc>,
}

async fn contracts(State(st): State<AppState>) -> Json<Vec<ContractRow>> {
    let rows = sqlx::query_as!(
        ContractRow,
        r#"SELECT name, address, chain_id, deployed_at as "deployed_at!" FROM contracts ORDER BY deployed_at DESC LIMIT 50"#
    )
    .fetch_all(&st.db)
    .await
    .unwrap_or_default();
    Json(rows)
}

#[derive(Serialize)]
struct MetricRow {
    key: String,
    value: serde_json::Value,
    updated_at: chrono::DateTime<chrono::Utc>,
}

async fn outlets(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT outlet_id, owner, name, domain, created_at FROM outlets ORDER BY created_at DESC LIMIT 200"#
    ).fetch_all(&st.db).await.unwrap_or_default();
    Json(rows.into_iter().map(|r| serde_json::json!(r)).collect())
}

async fn outlet_tokens(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT token_address, owner, name, symbol, supply, deployed_at FROM outlet_tokens ORDER BY deployed_at DESC LIMIT 200"#
    ).fetch_all(&st.db).await.unwrap_or_default();
    Json(rows.into_iter().map(|r| serde_json::json!(r)).collect())
}

async fn articles(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT article_id, outlet_id, author, uri, content_hash, created_at FROM articles ORDER BY created_at DESC LIMIT 200"#
    ).fetch_all(&st.db).await.unwrap_or_default();
    Json(rows.into_iter().map(|r| serde_json::json!(r)).collect())
}

async fn proposals(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT proposal_id, proposer, proposal_type, title, description_uri, created_at, ends_at, fee_paid FROM proposals ORDER BY created_at DESC LIMIT 200"#
    ).fetch_all(&st.db).await.unwrap_or_default();
    Json(rows.into_iter().map(|r| serde_json::json!(r)).collect())
}

async fn params(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query!(r#"SELECT key, value, updated_at FROM params ORDER BY updated_at DESC LIMIT 200"#)
        .fetch_all(&st.db).await.unwrap_or_default();
    Json(rows.into_iter().map(|r| serde_json::json!(r)).collect())
}

async fn bonds(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query!(r#"SELECT account, role, amount, updated_at FROM bonds ORDER BY updated_at DESC LIMIT 500"#)
        .fetch_all(&st.db).await.unwrap_or_default();
    Json(rows.into_iter().map(|r| serde_json::json!(r)).collect())
}

async fn proposal_votes(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query!(r#"SELECT proposal_id, voter, support, weight, created_at FROM proposal_votes ORDER BY created_at DESC LIMIT 500"#)
        .fetch_all(&st.db).await.unwrap_or_default();
    Json(rows.into_iter().map(|r| serde_json::json!(r)).collect())
}

async fn governance_overview(State(st): State<AppState>) -> Json<serde_json::Value> {
    let proposals = sqlx::query!(r#"SELECT COUNT(*)::BIGINT as n FROM proposals"#).fetch_one(&st.db).await.ok().and_then(|r| r.n).unwrap_or(0);
    let active_council = sqlx::query!(r#"SELECT COUNT(*)::BIGINT as n FROM council_members WHERE active=true"#).fetch_one(&st.db).await.ok().and_then(|r| r.n).unwrap_or(0);
    let votes = sqlx::query!(r#"SELECT COUNT(*)::BIGINT as n FROM proposal_votes"#).fetch_one(&st.db).await.ok().and_then(|r| r.n).unwrap_or(0);
    let params = sqlx::query!(r#"SELECT COUNT(*)::BIGINT as n FROM params"#).fetch_one(&st.db).await.ok().and_then(|r| r.n).unwrap_or(0);

    Json(serde_json::json!({
        "proposals": proposals,
        "activeCouncil": active_council,
        "votes": votes,
        "params": params
    }))
}

async fn council_votes(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query!(r#"SELECT proposal_id, council, support, created_at FROM council_votes ORDER BY created_at DESC LIMIT 500"#)
        .fetch_all(&st.db).await.unwrap_or_default();
    Json(rows.into_iter().map(|r| serde_json::json!(r)).collect())
}

async fn council_members(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query!(r#"SELECT member, active, term_start, term_end, last_activity, removal_reason, removed_at, updated_at FROM council_members ORDER BY updated_at DESC LIMIT 500"#)
        .fetch_all(&st.db).await.unwrap_or_default();
    Json(rows.into_iter().map(|r| serde_json::json!(r)).collect())
}

async fn proposal_lifecycle(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query!(r#"SELECT proposal_id, event_type, data, created_at FROM proposal_lifecycle ORDER BY created_at DESC LIMIT 500"#)
        .fetch_all(&st.db).await.unwrap_or_default();
    Json(rows.into_iter().map(|r| serde_json::json!(r)).collect())
}

async fn multisig_txs(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query!(r#"SELECT tx_id, target, value, approvals, status, updated_at FROM multisig_txs ORDER BY updated_at DESC LIMIT 200"#)
        .fetch_all(&st.db).await.unwrap_or_default();
    Json(rows.into_iter().map(|r| serde_json::json!(r)).collect())
}

async fn court_cases(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query!(
        r#"SELECT case_id, outlet_id, filed_by, case_type, evidence_uri, created_at, status FROM court_cases ORDER BY created_at DESC LIMIT 200"#
    ).fetch_all(&st.db).await.unwrap_or_default();
    Json(rows.into_iter().map(|r| serde_json::json!(r)).collect())
}

async fn metrics(State(st): State<AppState>) -> Json<Vec<MetricRow>> {
    let rows = sqlx::query_as!(
        MetricRow,
        r#"SELECT key, value, updated_at as "updated_at!" FROM chain_metrics ORDER BY updated_at DESC LIMIT 50"#
    )
    .fetch_all(&st.db)
    .await
    .unwrap_or_default();
    Json(rows)
}

#[tokio::main]
async fn main() {
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let port: u16 = std::env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8787);

    let db = PgPool::connect(&db_url).await.expect("db connect");
    let st = AppState { db };

    
#[derive(serde::Deserialize)]

async fn 
async fn init_outlet_pools_schema(state: &AppState) {
    let _ = sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS outlets (
            outlet_id TEXT PRIMARY KEY,
            token_address TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
    "#).execute(&state.db).await;

    let _ = sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS outlet_members (
            outlet_id TEXT NOT NULL,
            wallet TEXT NOT NULL,
            joined_at INTEGER NOT NULL,
            PRIMARY KEY (outlet_id, wallet)
        );
    "#).execute(&state.db).await;

    let _ = sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS outlet_pool (
            outlet_id TEXT PRIMARY KEY,
            balance_wei TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );
    "#).execute(&state.db).await;
}

init_governance_schema(state: &AppState) {
    // Council policy config (single row)
    let _ = sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS council_policy (
            id INTEGER PRIMARY KEY CHECK (id=1),
            term_days INTEGER NOT NULL,
            min_actions_30d INTEGER NOT NULL,
            bond_min_wei TEXT NOT NULL
        );
    "#).execute(&state.db).await;

    let _ = sqlx::query(r#"
        INSERT INTO council_policy(id, term_days, min_actions_30d, bond_min_wei)
        VALUES (1, 180, 25, "0")
        ON CONFLICT(id) DO NOTHING;
    "#).execute(&state.db).await;

    // Per-wallet council term start (unix ts). Term validity = start + term_days.
    let _ = sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS council_terms (
            wallet TEXT PRIMARY KEY,
            term_start INTEGER NOT NULL
        );
    "#).execute(&state.db).await;

    // Rolling activity cache for governance (updated by indexer). Stores actions in last 30d and last_action ts.
    let _ = sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS council_activity (
            wallet TEXT PRIMARY KEY,
            actions_30d INTEGER NOT NULL,
            last_action INTEGER NOT NULL
        );
    "#).execute(&state.db).await;

    // Bond cache (updated by indexer/contract reads). Stored as wei string.
    let _ = sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS council_bonds (
            wallet TEXT PRIMARY KEY,
            bond_wei TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );
    "#).execute(&state.db).await;
}

struct CouncilEligibleQ {
    wallet: String,
}

#[derive(serde::Serialize)]
struct CouncilEligibleResp {
    eligible: bool,
    reason: String,
    wallet: String,
    term_ok: bool,
    activity_ok: bool,
    bond_ok: bool,
}

// Council eligibility hook endpoint.
// MVP logic: eligible if wallet has on-chain role COUNCIL (via existing roles handler).
// In future: enforce 6-month term, activity, and bond thresholds using indexed state.

#[derive(serde::Deserialize)]
struct CouncilPolicySetReq {
    term_days: i64,
    min_actions_30d: i64,
    bond_min_wei: String,
}

async fn admin_set_council_policy(State(state): State<AppState>, headers: axum::http::HeaderMap, Json(req): Json<CouncilPolicySetReq>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_headers(&state, &headers) { return Err(StatusCode::UNAUTHORIZED); }
    let _ = sqlx::query("UPDATE council_policy SET term_days=?1, min_actions_30d=?2, bond_min_wei=?3 WHERE id=1")
        .bind(req.term_days).bind(req.min_actions_30d).bind(req.bond_min_wei).execute(&state.db).await;
    Ok(Json(serde_json::json!({"ok": true})))
}

#[derive(serde::Deserialize)]
struct WalletReq { wallet: String }

#[derive(serde::Deserialize)]
struct CouncilTermSetReq { wallet: String, term_start: i64 }

async fn admin_set_council_term(State(state): State<AppState>, headers: axum::http::HeaderMap, Json(req): Json<CouncilTermSetReq>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_headers(&state, &headers) { return Err(StatusCode::UNAUTHORIZED); }
    let _ = sqlx::query("INSERT INTO council_terms(wallet,term_start) VALUES (?1,?2) ON CONFLICT(wallet) DO UPDATE SET term_start=excluded.term_start")
        .bind(req.wallet).bind(req.term_start).execute(&state.db).await;
    Ok(Json(serde_json::json!({"ok": true})))
}

#[derive(serde::Deserialize)]
struct CouncilActivitySetReq { wallet: String, actions_30d: i64, last_action: i64 }

async fn admin_set_council_activity(State(state): State<AppState>, headers: axum::http::HeaderMap, Json(req): Json<CouncilActivitySetReq>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_headers(&state, &headers) { return Err(StatusCode::UNAUTHORIZED); }
    let _ = sqlx::query("INSERT INTO council_activity(wallet,actions_30d,last_action) VALUES (?1,?2,?3) ON CONFLICT(wallet) DO UPDATE SET actions_30d=excluded.actions_30d, last_action=excluded.last_action")
        .bind(req.wallet).bind(req.actions_30d).bind(req.last_action).execute(&state.db).await;
    Ok(Json(serde_json::json!({"ok": true})))
}

#[derive(serde::Deserialize)]
struct CouncilBondSetReq { wallet: String, bond_wei: String }

async fn admin_set_council_bond(State(state): State<AppState>, headers: axum::http::HeaderMap, Json(req): Json<CouncilBondSetReq>) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_headers(&state, &headers) { return Err(StatusCode::UNAUTHORIZED); }
    let now = now_ts();
    let _ = sqlx::query("INSERT INTO council_bonds(wallet,bond_wei,updated_at) VALUES (?1,?2,?3) ON CONFLICT(wallet) DO UPDATE SET bond_wei=excluded.bond_wei, updated_at=excluded.updated_at")
        .bind(req.wallet).bind(req.bond_wei).bind(now).execute(&state.db).await;
    Ok(Json(serde_json::json!({"ok": true})))
}

fn is_admin_headers(state: &AppState, headers: &axum::http::HeaderMap) -> bool {
    let tok = headers.get("x-admin-token").and_then(|v| v.to_str().ok()).unwrap_or("");
    let cfg = state.cfg.blocking_read().clone();
    tok == cfg.admin_token
}


fn compare_wei_ge(a: &str, b: &str) -> bool {
    // a >= b for non-negative integer strings
    let a = a.trim_start_matches('0');
    let b = b.trim_start_matches('0');
    let a = if a.is_empty() { "0" } else { a };
    let b = if b.is_empty() { "0" } else { b };
    if a.len() != b.len() { return a.len() > b.len(); }
    a >= b
}



#[derive(serde::Deserialize)]
struct OutletQ { outlet_id: String }

#[derive(serde::Serialize)]
struct OutletPoolStatus {
    outlet_id: String,
    balance_wei: String,
    members: usize,
}

async fn outlet_pool_status(State(state): State<AppState>, axum::extract::Query(q): axum::extract::Query<OutletQ>) -> Json<OutletPoolStatus> {
    let bal = sqlx::query("SELECT balance_wei FROM outlet_pool WHERE outlet_id=?1")
        .bind(&q.outlet_id).fetch_optional(&state.db).await.ok().flatten()
        .map(|r| r.get::<String,_>(0)).unwrap_or("0".into());
    let members = sqlx::query("SELECT COUNT(*) FROM outlet_members WHERE outlet_id=?1")
        .bind(&q.outlet_id).fetch_one(&state.db).await.ok().map(|r| r.get::<i64,_>(0) as usize).unwrap_or(0);
    Json(OutletPoolStatus{ outlet_id: q.outlet_id, balance_wei: bal, members })
}

async fn council_eligible(State(state): State<AppState>, axum::extract::Query(q): axum::extract::Query<CouncilEligibleQ>) -> Json<CouncilEligibleResp> {
    let wallet = q.wallet.clone();

    // Base: wallet must have COUNCIL role on-chain
    let roles = state.role_store.roles_of(&wallet).await.unwrap_or_default();
    let has_role = roles.iter().any(|r| r == "COUNCIL");

    // Load policy
    let pol = sqlx::query("SELECT term_days, min_actions_30d, bond_min_wei FROM council_policy WHERE id=1")
        .fetch_optional(&state.db).await.ok().flatten();
    let term_days:i64 = pol.as_ref().map(|r| r.get::<i64,_>(0)).unwrap_or(180);
    let min_actions_30d:i64 = pol.as_ref().map(|r| r.get::<i64,_>(1)).unwrap_or(25);
    let bond_min_wei:String = pol.as_ref().map(|r| r.get::<String,_>(2)).unwrap_or("0".into());

    let now = now_ts();

    // Term: if no term_start exists yet but has_role, auto-set term_start = now (MVP convenience).
    let mut term_ok = false;
    if has_role {
        let term_row = sqlx::query("SELECT term_start FROM council_terms WHERE wallet=?1")
            .bind(&wallet).fetch_optional(&state.db).await.ok().flatten();
        let term_start:i64 = if let Some(r)=term_row { r.get(0) } else {
            let _ = sqlx::query("INSERT INTO council_terms(wallet,term_start) VALUES (?1,?2)")
                .bind(&wallet).bind(now).execute(&state.db).await;
            now
        };
        term_ok = now <= term_start + (term_days * 86400);
    }

    // Activity: expects indexer to maintain actions_30d. If missing and has_role, treat as not ok (forces real activity tracking).
    let act_row = sqlx::query("SELECT actions_30d, last_action FROM council_activity WHERE wallet=?1")
        .bind(&wallet).fetch_optional(&state.db).await.ok().flatten();
    let mut activity_ok = false;
    let mut last_action = 0i64;
    let mut actions_30d = 0i64;
    if let Some(r)=act_row {
        actions_30d = r.get(0);
        last_action = r.get(1);
        activity_ok = actions_30d >= min_actions_30d;
    } else if has_role {
        activity_ok = false;
    }

    // Bond: expects indexer/contract read to maintain bond_wei. If missing, treat as not ok when policy requires >0.
    let bond_row = sqlx::query("SELECT bond_wei FROM council_bonds WHERE wallet=?1")
        .bind(&wallet).fetch_optional(&state.db).await.ok().flatten();
    let mut bond_ok = false;
    let mut bond_wei = "0".to_string();
    if let Some(r)=bond_row {
        bond_wei = r.get(0);
        // Compare as big integers in string form (MVP: length + lex compare)
        bond_ok = compare_wei_ge(&bond_wei, &bond_min_wei);
    } else {
        bond_ok = bond_min_wei == "0";
    }

    let eligible = has_role && term_ok && activity_ok && bond_ok;

    let mut reason = String::new();
    if !has_role { reason = "missing_council_role".into(); }
    else if !term_ok { reason = "term_expired".into(); }
    else if !activity_ok { reason = format!("insufficient_activity:{}_of_required_{}", actions_30d, min_actions_30d); }
    else if !bond_ok { reason = "bond_below_minimum".into(); }
    else { reason = "eligible".into(); }

    Json(CouncilEligibleResp{
        eligible,
        reason,
        wallet,
        term_ok,
        activity_ok,
        bond_ok,
    })
}


let app = Router::new()
        .route("/outlet/pool/status", get(outlet_pool_status))
        .route("/council/eligible", get(council_eligible))
        .route("/health", get(health))
        .route("/api/search", get(search))
        .route("/api/feed/:feed", get(feed))
        .route("/v1/contracts", get(contracts))
        .route("/v1/metrics", get(metrics))
        .route("/v1/outlets", get(outlets))
        .route("/v1/outlet_tokens", get(outlet_tokens))
        .route("/v1/articles", get(articles))
        .route("/v1/proposals", get(proposals))
        .route("/v1/court_cases", get(court_cases))
        .route("/v1/params", get(params))
        .route("/v1/bonds", get(bonds))
        .route("/v1/proposal_votes", get(proposal_votes))
        .route("/v1/multisig_txs", get(multisig_txs))
        .route("/v1/proposal_lifecycle", get(proposal_lifecycle))
        .route("/v1/council_members", get(council_members))
        .route("/v1/council_votes", get(council_votes))
        .route("/v1/governance_overview", get(governance_overview))
        .layer(CorsLayer::permissive())
        .with_state(st);

    let addr = SocketAddr::from(([0,0,0,0], port));
    println!("press_query_api listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app).await.unwrap();
}


// --- Search + Feeds (RR83) ---
// Minimal API to support bots and UIs. Next passes will query the real index.

use axum::extract::{Query, Path};

#[derive(Deserialize)]
struct SearchQ { q: String }

async fn search(Query(q): Query<SearchQ>) -> Json<serde_json::Value> {
    let idx = std::env::var("PRESS_INDEXER_API").unwrap_or_else(|_| "http://press-indexer:8786".into());
    let url = format!("{}/search?q={}", idx, urlencoding::encode(&q.q));
    if let Ok(r) = reqwest::Client::new().get(url).send().await {
        if let Ok(j) = r.json::<serde_json::Value>().await {
            return Json(j);
        }
    }
    Json(serde_json::json!({"ok": true, "query": q.q, "items": []}))
}

    // Placeholder: return empty list with echo; real implementation queries indexer db.
    Json(serde_json::json!({"ok": true, "query": q.q, "items": []}))
}

#[derive(Deserialize)]
struct FeedQ { after: Option<i64>, outlet: Option<String>, article_id: Option<String>, kind: Option<String>, severity: Option<i64> }

async fn feed(Path(feed): Path<String>, Query(q): Query<FeedQ>) -> Json<serde_json::Value> {
    let after = q.after.unwrap_or(0);
    let idx = std::env::var("PRESS_INDEXER_API").unwrap_or_else(|_| "http://press-indexer:8786".into());
    let mut url = format!("{}/feed/{}?after={}", idx, feed, after);
    if let Some(o) = q.outlet.as_ref() { url.push_str(&format!("&outlet={}", urlencoding::encode(o))); }
    if let Some(a) = q.article_id.as_ref() { url.push_str(&format!("&article_id={}", urlencoding::encode(a))); }
    if let Some(k) = q.kind.as_ref() { url.push_str(&format!("&kind={}", urlencoding::encode(k))); }
    if let Some(sv) = q.severity { url.push_str(&format!("&severity={}", sv)); }

    if let Ok(r) = reqwest::Client::new().get(url).send().await {
        if let Ok(j) = r.json::<serde_json::Value>().await {
            return Json(j);
        }
    }
    Json(serde_json::json!({"ok": true, "feed": feed, "after": after, "items": []}))
}

    let after = q.after.unwrap_or(0);
    Json(serde_json::json!({"ok": true, "feed": feed, "after": after, "items": []}))
}
