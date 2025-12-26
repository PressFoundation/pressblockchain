use axum::{routing::get, routing::post, Json, Router, extract::{State, Query}};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{SqlitePool, Row};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
    rpc_http: String,
}

#[derive(Deserialize)]
struct DeployJson {
    #[serde(default)]
    outletRegistry: String,
    #[serde(default)]
    outletTokenFactory: String,
    #[serde(default)]
    exchangeListingRegistry: String,
}

fn k256(sig: &str) -> String {
    let mut hasher = sha3::Keccak256::new();
    use sha3::Digest;
    hasher.update(sig.as_bytes());
    format!("0x{}", hex::encode(hasher.finalize()))
}

fn hex_to_bytes(h: &str) -> Vec<u8> {
    let s = h.strip_prefix("0x").unwrap_or(h);
    hex::decode(s).unwrap_or_default()
}
async fn health() -> Json<serde_json::Value> { Json(serde_json::json!({"ok": true})) }

fn u256_at(data_hex: &str, slot: usize) -> u128 {
    let b = hex_to_bytes(data_hex);
    let start = slot * 32;
    if b.len() < start + 32 { return 0; }
    // take last 16 bytes to fit u128
    let mut v: u128 = 0;
    for x in &b[start+16..start+32] {
        v = (v << 8) | (*x as u128);
    }
    v
}
fn bytes32_at_topic(t: &str) -> String { t.to_string() }

fn addr_from_topic(t: &str) -> String {
    let s = t.strip_prefix("0x").unwrap_or(t);
    if s.len() < 64 { return format!("0x{}", s); }
    format!("0x{}", &s[24..64])
}

fn decode_string(data_hex: &str, head_slot: usize) -> String {
    // ABI: head contains offset (bytes) from start of data
    let b = hex_to_bytes(data_hex);
    if b.len() < (head_slot+1)*32 { return "".into(); }
    let mut off: usize = 0;
    for x in &b[head_slot*32..head_slot*32+32] { off = (off<<8) | (*x as usize); }
    if b.len() < off + 32 { return "".into(); }
    let mut len: usize = 0;
    for x in &b[off..off+32] { len = (len<<8) | (*x as usize); }
    if b.len() < off + 32 + len { return "".into(); }
    String::from_utf8_lossy(&b[off+32..off+32+len]).to_string()
}

#[derive(Deserialize)]
struct RpcReq {
    jsonrpc: String,
    id: u32,
    method: String,
    params: serde_json::Value,
}

async fn rpc_call(rpc: &str, method: &str, params: serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let req = RpcReq{ jsonrpc:"2.0".into(), id:1, method:method.into(), params };
    let v: serde_json::Value = client.post(rpc).json(&req).send().await?.json().await?;
    Ok(v.get("result").cloned().unwrap_or(json!(null)))
}

async fn ensure_schema(db: &PgPool) {
    let _ = sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS meta (
            k TEXT PRIMARY KEY,
            v TEXT NOT NULL
        );
    "#).execute(db).await;

    let _ = sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS outlets (
            block_number BIGINT NOT NULL,
            tx_hash TEXT NOT NULL,
            outlet_id TEXT NOT NULL,
            owner TEXT NOT NULL,
            name TEXT NOT NULL,
            domain TEXT NOT NULL,
            bond_paid BIGINT NOT NULL,
            fee_paid BIGINT NOT NULL,
            inserted_at TEXT NOT NULL
        );
    "#).execute(db).await;

    let _ = sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS outlet_tokens (
            block_number BIGINT NOT NULL,
            tx_hash TEXT NOT NULL,
            outlet_id TEXT NOT NULL,
            token TEXT NOT NULL,
            owner TEXT NOT NULL,
            name TEXT NOT NULL,
            symbol TEXT NOT NULL,
            supply BIGINT NOT NULL,
            fee_paid BIGINT NOT NULL,
            inserted_at TEXT NOT NULL
        );
    "#).execute(db).await;

    let _ = sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS token_listings (
            block_number BIGINT NOT NULL,
            tx_hash TEXT NOT NULL,
            token TEXT NOT NULL,
            outlet_id TEXT NOT NULL,
            owner TEXT NOT NULL,
            tier BIGINT NOT NULL,
            fee_paid BIGINT NOT NULL,
            perks TEXT NOT NULL,
            inserted_at TEXT NOT NULL
        );
    "#).execute(db).await;

    let _ = sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS outlet_domain_verifications (
            block_number BIGINT NOT NULL,
            tx_hash TEXT NOT NULL,
            outlet_id TEXT NOT NULL,
            domain TEXT NOT NULL,
            proof_type BIGINT NOT NULL,
            proof_hash TEXT NOT NULL,
            verifier TEXT NOT NULL,
            inserted_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS exchange_listings (
            outlet TEXT PRIMARY KEY,
            tier TEXT NOT NULL,
            domain TEXT NOT NULL,
            fee_paid TEXT NOT NULL,
            test_passed BIGINT NOT NULL,
            listed_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS governance_proposals (
            proposal_id BIGINT PRIMARY KEY,
            proposer TEXT NOT NULL,
            title TEXT NOT NULL,
            config_key TEXT NOT NULL,
            config_value TEXT NOT NULL,
            fee_paid TEXT NOT NULL,
            created_at TEXT NOT NULL,
            ends_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS approved_updates (
            proposal_id BIGINT NOT NULL,
            config_key TEXT NOT NULL,
            config_value TEXT NOT NULL,
            passed BIGINT NOT NULL,
            auto_applied BIGINT NOT NULL,
            reason TEXT NOT NULL,
            recorded_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS release_batch_items (
            batch_id TEXT NOT NULL,
            proposal_id BIGINT NOT NULL,
            config_key TEXT NOT NULL,
            config_value TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE (batch_id, proposal_id)
        );

        CREATE TABLE IF NOT EXISTS release_batches (
            batch_id TEXT NOT NULL,
            proposal_id BIGINT NOT NULL,
            config_key TEXT NOT NULL,
            config_value TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS release_batches (
            batch_id TEXT NOT NULL,
            title TEXT NOT NULL,
            window_start TEXT NOT NULL,
            window_end TEXT NOT NULL,
            status TEXT NOT NULL,
            notes TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS token_tests (
            token TEXT NOT NULL,
            outlet_id TEXT NOT NULL,
            tx_hash TEXT NOT NULL,
            symbol TEXT NOT NULL,
            status BIGINT NOT NULL,
            tested_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS outlet_domain_checks (
            outlet_id TEXT NOT NULL,
            domain TEXT NOT NULL,
            dns_ok BOOLEAN NOT NULL,
            http_ok BOOLEAN NOT NULL,
            notes TEXT NOT NULL,
            checked_at TEXT NOT NULL
        );
    "#).execute(db).await;

    // last block marker
    let _ = sqlx::query("INSERT INTO meta (k,v) VALUES ('last_block','0') ON CONFLICT (k) DO NOTHING;")
        .execute(db).await;
}

fn now_iso() -> String { chrono::Utc::now().to_rfc3339() }

async fn read_last_block(db: &PgPool) -> i64 {
    sqlx::query("SELECT v FROM meta WHERE k='last_block'").fetch_one(db).await
        .ok()
        .and_then(|r| r.try_get::<String,_>("v").ok())
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
}
async fn write_last_block(db: &PgPool, n: i64) {
    let _ = sqlx::query("UPDATE meta SET v=$1 WHERE k='last_block'")
        .bind(n.to_string()).execute(db).await;
}

#[derive(Deserialize)]
struct LogEntry {
    address: String,
    topics: Vec<String>,
    data: String,
    #[serde(rename="blockNumber")]
    block_number: String,
    #[serde(rename="transactionHash")]
    tx_hash: String,
}

async fn poll_loop(st: AppState) {
    let topic_outlet_created = k256("OutletCreated(bytes32,address,string,string,uint256,uint256)");
    let topic_domain_verified = k256("DomainVerified(bytes32,string,uint8,bytes32,address)");
    let topic_outlet_token_deployed = k256("OutletTokenDeployed(bytes32,address,address,string,string,uint256,uint256)");
    let topic_token_listed = k256("TokenListed(address,bytes32,address,uint8,uint256,uint256)");
    let topic_heartbeat = k256("Heartbeat(bytes32,address,uint64,uint8,bytes32)");
    loop {
        let last = read_last_block(&st.db).await;
        // ask latest block
        let latest_hex = rpc_call(&st.rpc_http, "eth_blockNumber", json!([])).await.ok().and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or("0x0".into());
        let latest = i64::from_str_radix(latest_hex.trim_start_matches("0x"), 16).unwrap_or(0);
        let from = if last == 0 { latest.saturating_sub(3000) } else { last.saturating_sub(0) };
        let to = latest;

        // load deploy addresses
        let deploy: DeployJson = std::fs::read_to_string("/state/deploy.json").ok()
            .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(DeployJson{outletRegistry:"".into(), outletTokenFactory:"".into(), exchangeListingRegistry:"".into()});
        let mut addrs: Vec<String> = vec![];
        if !deploy.outletRegistry.is_empty() { addrs.push(deploy.outletRegistry); }
        if !deploy.outletTokenFactory.is_empty() { addrs.push(deploy.outletTokenFactory); }
        if !deploy.exchangeListingRegistry.is_empty() { addrs.push(deploy.exchangeListingRegistry); }
        if !deploy.uptimeBeacon.is_empty() { addrs.push(deploy.uptimeBeacon); }
        if addrs.is_empty() {
            tokio::time::sleep(Duration::from_secs(3)).await;
            continue;
        }

        let filter = json!([{
            "fromBlock": format!("0x{:x}", from),
            "toBlock": format!("0x{:x}", to),
            "address": addrs,
            "topics": [[topic_outlet_created.clone(), topic_outlet_token_deployed.clone(), topic_token_listed.clone(), topic_domain_verified.clone()]]
        }]);

        let res = rpc_call(&st.rpc_http, "eth_getLogs", filter).await;
        if let Ok(val) = res {
            if let Some(arr) = val.as_array() {
                for item in arr {
                    if let Ok(lg) = serde_json::from_value::<LogEntry>(item.clone()) {
                        let bn = i64::from_str_radix(lg.block_number.trim_start_matches("0x"), 16).unwrap_or(0);
                        let t0 = lg.topics.get(0).cloned().unwrap_or_default();
                        let inserted_at = now_iso();
                        
if t0 == topic_heartbeat {
    // Heartbeat(service, caller, ts, status, extra)
    let service = lg.topics.get(1).cloned().unwrap_or_default();
    let data = hex_to_bytes(&lg.data);
    // data layout: ts(uint64) padded 32, status(uint8) padded 32, extra(bytes32)
    let ts = if data.len() >= 32 {
        let mut b=[0u8;8];
        b.copy_from_slice(&data[24..32]);
        u64::from_be_bytes(b) as i64
    } else { 0 };
    let status = if data.len() >= 64 { data[63] as i64 } else { 0 };
    let extra = lg.data.clone();
    let tx_hash = lg.tx_hash.clone();
    // store
    let _ = sqlx::query(r#"INSERT INTO heartbeats(block_number, tx_hash, service, ts, status, extra) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#)
        .bind(bn)
        .bind(tx_hash)
        .bind(service)
        .bind(ts)
        .bind(status)
        .bind(extra)
        .execute(&st.db)
        .await;
    continue;
}

if t0 == topic_outlet_created && lg.topics.len() >= 3 {
                            let outlet_id = bytes32_at_topic(&lg.topics[1]);
                            let owner = addr_from_topic(&lg.topics[2]);
                            let name = decode_string(&lg.data, 0);
                            let domain = decode_string(&lg.data, 1);
                            let bond = u256_at(&lg.data, 2) as i64;
                            let fee = u256_at(&lg.data, 3) as i64;
                            let _ = sqlx::query("INSERT INTO outlets (block_number, tx_hash, outlet_id, owner, name, domain, bond_paid, fee_paid, inserted_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)")
                                .bind(bn).bind(&lg.tx_hash).bind(&outlet_id).bind(&owner).bind(&name).bind(&domain).bind(bond).bind(fee).bind(&inserted_at)
                                .execute(&st.db).await;
                        } else if t0 == topic_outlet_token_deployed && lg.topics.len() >= 4 {
                            // topics: [sig, outletId, token, owner]
                            let outlet_id = bytes32_at_topic(&lg.topics[1]);
                            let token = addr_from_topic(&lg.topics[2]);
                            let owner = addr_from_topic(&lg.topics[3]);
                            let name = decode_string(&lg.data, 0);
                            let symbol = decode_string(&lg.data, 1);
                            let supply = u256_at(&lg.data, 2) as i64;
                            let fee = u256_at(&lg.data, 3) as i64;
                            let _ = sqlx::query("INSERT INTO outlet_tokens (block_number, tx_hash, outlet_id, token, owner, name, symbol, supply, fee_paid, inserted_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)")
                                .bind(bn).bind(&lg.tx_hash).bind(&outlet_id).bind(&token).bind(&owner).bind(&name).bind(&symbol).bind(supply).bind(fee).bind(&inserted_at)
                                .execute(&st.db).await;
                        } else if t0 == topic_domain_verified && lg.topics.len() >= 3 {
    // topics: [sig, outletId, verifier]
    let outlet_id = bytes32_at_topic(&lg.topics[1]);
    let verifier = addr_from_topic(&lg.topics[2]);
    let domain = decode_string(&lg.data, 0);
    let proof_type = u256_at(&lg.data, 1) as i64;
    // proof hash is bytes32 in slot 2 (full 32 bytes)
    let b = hex_to_bytes(&lg.data);
    let start = 2*32;
    let proof_hash = if b.len()>=start+32 { format!("0x{}", hex::encode(&b[start..start+32])) } else { "0x".into() };
    let _ = sqlx::query("INSERT INTO outlet_domain_verifications (block_number, tx_hash, outlet_id, domain, proof_type, proof_hash, verifier, inserted_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(bn).bind(&lg.tx_hash).bind(&outlet_id).bind(&domain).bind(proof_type).bind(&proof_hash).bind(&verifier).bind(&inserted_at)
        .execute(&st.db).await;
                        } else if t0 == topic_token_listed && lg.topics.len() >= 4 {
                            // topics: [sig, token, outletId, owner]
                            let token = addr_from_topic(&lg.topics[1]);
                            let outlet_id = bytes32_at_topic(&lg.topics[2]);
                            let owner = addr_from_topic(&lg.topics[3]);
                            let tier = u256_at(&lg.data, 0) as i64;
                            let fee = u256_at(&lg.data, 1) as i64;
                            let perks = format!("{}", u256_at(&lg.data, 2));
                            let _ = sqlx::query("INSERT INTO token_listings (block_number, tx_hash, token, outlet_id, owner, tier, fee_paid, perks, inserted_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)")
                                .bind(bn).bind(&lg.tx_hash).bind(&token).bind(&outlet_id).bind(&owner).bind(tier).bind(fee).bind(&perks).bind(&inserted_at)
                                .execute(&st.db).await;
                        }
                        if bn > last { write_last_block(&st.db, bn).await; }
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

#[derive(Serialize)]
struct SimpleRow { kind: String, block_number: i64, tx_hash: String, fields: serde_json::Value, inserted_at: String }

async fn decoded_latest(State(st): State<AppState>) -> Json<Vec<SimpleRow>> {
    let mut out: Vec<SimpleRow> = vec![];
    let rows = sqlx::query("SELECT block_number, tx_hash, outlet_id, owner, name, domain, bond_paid, fee_paid, inserted_at FROM outlets ORDER BY block_number DESC LIMIT 50")
        .fetch_all(&st.db).await.unwrap_or_default();
    for r in rows {
        out.push(SimpleRow{
            kind:"outlet_created".into(),
            block_number:r.get::<i64,_>("block_number"),
            tx_hash:r.get::<String,_>("tx_hash"),
            fields: json!({
                "outlet_id": r.get::<String,_>("outlet_id"),
                "owner": r.get::<String,_>("owner"),
                "name": r.get::<String,_>("name"),
                "domain": r.get::<String,_>("domain"),
                "official_url": format!("https://{}", r.get::<String,_>("domain")),
                "bond_paid": r.get::<i64,_>("bond_paid"),
                "fee_paid": r.get::<i64,_>("fee_paid"),
            }),
            inserted_at:r.get::<String,_>("inserted_at"),
        });
    }
    let rows = sqlx::query("SELECT block_number, tx_hash, token, outlet_id, owner, tier, fee_paid, perks, inserted_at FROM token_listings ORDER BY block_number DESC LIMIT 50")
        .fetch_all(&st.db).await.unwrap_or_default();
    for r in rows {
        out.push(SimpleRow{
            kind:"token_listed".into(),
            block_number:r.get::<i64,_>("block_number"),
            tx_hash:r.get::<String,_>("tx_hash"),
            fields: json!({
                "token": r.get::<String,_>("token"),
                "outlet_id": r.get::<String,_>("outlet_id"),
                "owner": r.get::<String,_>("owner"),
                "tier": r.get::<i64,_>("tier"),
                "fee_paid": r.get::<i64,_>("fee_paid"),
                "perks": r.get::<String,_>("perks"),
            }),
            inserted_at:r.get::<String,_>("inserted_at"),
        });
    }
    Json(out)
}

async fn outlets_latest(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT block_number, tx_hash, outlet_id, owner, name, domain, bond_paid, fee_paid, inserted_at FROM outlets ORDER BY block_number DESC LIMIT 200")
        .fetch_all(&st.db).await.unwrap_or_default();
    let out = rows.into_iter().map(|r| json!({
        "block_number": r.get::<i64,_>("block_number"),
        "tx_hash": r.get::<String,_>("tx_hash"),
        "outlet_id": r.get::<String,_>("outlet_id"),
        "owner": r.get::<String,_>("owner"),
        "name": r.get::<String,_>("name"),
        "domain": r.get::<String,_>("domain"),
        "official_url": format!("https://{}", r.get::<String,_>("domain")),
        "bond_paid": r.get::<i64,_>("bond_paid"),
        "fee_paid": r.get::<i64,_>("fee_paid"),
        "inserted_at": r.get::<String,_>("inserted_at"),
            "token_test": {
              "tx_hash": r.try_get::<String,_>("tx_hash").ok(),
              "symbol": r.try_get::<String,_>("symbol").ok(),
              "status": r.try_get::<i64,_>("status").ok(),
              "tested_at": r.try_get::<String,_>("tested_at").ok()
            },
    })).collect();
    Json(out)
}

async fn listings_latest(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    // join outlet domain/name so exchange templates can show official url
    let rows = sqlx::query(r#"
        SELECT l.block_number, l.tx_hash, l.token, l.outlet_id, l.owner, l.tier, l.fee_paid, l.perks, l.inserted_at,
               o.name as outlet_name, o.domain as outlet_domain
        FROM token_listings l
        LEFT JOIN outlets o ON o.outlet_id = l.outlet_id
        LEFT JOIN LATERAL (
            SELECT tx_hash, symbol, status, tested_at
            FROM token_tests t
            WHERE t.token = l.token
            ORDER BY tested_at DESC
            LIMIT 1
        ) tt ON true
        ORDER BY l.block_number DESC
        LIMIT 300
    "#).fetch_all(&st.db).await.unwrap_or_default();
    let out = rows.into_iter().map(|r| {
        let dom: Option<String> = r.try_get("outlet_domain").ok();
        json!({
            "block_number": r.get::<i64,_>("block_number"),
            "tx_hash": r.get::<String,_>("tx_hash"),
            "token": r.get::<String,_>("token"),
            "outlet_id": r.get::<String,_>("outlet_id"),
            "owner": r.get::<String,_>("owner"),
            "tier": r.get::<i64,_>("tier"),
            "fee_paid": r.get::<i64,_>("fee_paid"),
            "perks": r.get::<String,_>("perks"),
            "outlet_name": r.try_get::<String,_>("outlet_name").ok(),
            "outlet_domain": dom.clone(),
            "official_url": dom.map(|d| format!("https://{}", d)),
            "inserted_at": r.get::<String,_>("inserted_at"),
            "token_test": {
              "tx_hash": r.try_get::<String,_>("tx_hash").ok(),
              "symbol": r.try_get::<String,_>("symbol").ok(),
              "status": r.try_get::<i64,_>("status").ok(),
              "tested_at": r.try_get::<String,_>("tested_at").ok()
            },
        })
    }).collect();
    Json(out)
}

#[derive(Deserialize)]
struct DomainCheckReq {
    outlet_id: String,
    domain: String,
    dns_ok: bool,
    http_ok: bool,
    #[serde(default)]
    notes: String,
}

async fn domain_check_write(State(st): State<AppState>, Json(req): Json<DomainCheckReq>) -> Json<serde_json::Value> {
    let _ = sqlx::query("INSERT INTO outlet_domain_checks (outlet_id, domain, dns_ok, http_ok, notes, checked_at) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(&req.outlet_id).bind(&req.domain).bind(req.dns_ok).bind(req.http_ok).bind(&req.notes).bind(now_iso())
        .execute(&st.db).await;
    Json(json!({"ok": true}))
}


#[derive(Deserialize)]
struct HeartbeatQuery { service: Option<String> }

async fn heartbeats_latest(State(st): State<AppState>, Query(q): Query<HeartbeatQuery>) -> Json<serde_json::Value> {
    let svc = q.service.unwrap_or_else(|| "press-bots".into());
    let row = sqlx::query("SELECT block_number, tx_hash, service, ts, status, extra FROM heartbeats WHERE service = ?1 ORDER BY ts DESC LIMIT 1")
        .bind(&svc)
        .fetch_optional(&st.db)
        .await
        .ok()
        .flatten();
    if let Some(r) = row {
        let block_number: i64 = r.get(0);
        let tx_hash: String = r.get(1);
        let service: String = r.get(2);
        let ts: i64 = r.get(3);
        let status: i64 = r.get(4);
        let extra: String = r.get(5);
        let now = chrono::Utc::now().timestamp();
        let age_sec = (now - ts).max(0);
        Json(serde_json::json!({"ok": true, "latest": {"block_number": block_number, "tx_hash": tx_hash, "service": service, "ts": ts, "status": status, "extra": extra}, "age_sec": age_sec}))
    } else {
        Json(serde_json::json!({"ok": false, "error": "no heartbeat found", "age_sec": 999999}))
    }
}

async fn domain_checks_latest(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT outlet_id, domain, dns_ok, http_ok, notes, checked_at FROM outlet_domain_checks ORDER BY checked_at DESC LIMIT 200")
        .fetch_all(&st.db).await.unwrap_or_default();
    let out = rows.into_iter().map(|r| json!({
        "outlet_id": r.get::<String,_>("outlet_id"),
        "domain": r.get::<String,_>("domain"),
        "official_url": format!("https://{}", r.get::<String,_>("domain")),
        "dns_ok": r.get::<bool,_>("dns_ok"),
        "http_ok": r.get::<bool,_>("http_ok"),
        "notes": r.get::<String,_>("notes"),
        "checked_at": r.get::<String,_>("checked_at"),
    })).collect();
    Json(out)
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://press:press@press-postgres:5432/press".into());
    let rpc_http = std::env::var("RPC_HTTP").unwrap_or_else(|_| "http://press-rpc:8545".into());
    let db = PgPool::connect(&db_url).await.expect("db");
    ensure_schema(&db).await;

    let st = AppState{ db: db.clone(), rpc_http };
    tokio::spawn(poll_loop(st.clone()));

    tokio::spawn(governance_ingest_loop(st.clone()));
    tokio::spawn(upgrade_queue_ingest_loop(st.clone()));
    tokio::spawn(exchange_registry_ingest_loop(st.clone()));

    let app = Router::new()
        .route("/health", get(health))
        .route("/search", get(search))
        .route("/feed/:feed", get(feed))
        .route("/events", axum::routing::post(post_event))
        .route("/oracle/flag", axum::routing::post(post_flag))
        .route("/decoded/latest", get(decoded_latest))
        .route("/outlets/latest", get(outlets_latest))
        .route("/listings/latest", get(listings_latest))
        .route("/domain_checks/latest", get(domain_checks_latest))
        .route("/heartbeats/latest", get(heartbeats_latest))
        .route("/domain_verifications/latest", get(domain_verifications_latest))
        .route("/domain_checks/write", post(domain_check_write))
        .route("/token_tests/write", post(token_test_write))
        .route("/token_tests/latest", get(token_tests_latest))
        .route("/governance/approved/write", post(approved_write))
        .route("/governance/approved/latest", get(approved_latest))
        .route("/governance/batches/write", post(batch_write))
        .route("/governance/batches/latest", get(batch_latest))
        .route("/governance/batch_items/latest", get(batch_items_latest))
        .route("/governance/vote_fees/latest", get(vote_fees_latest))
        .route("/governance/grants/latest", get(grants_latest))
        .route("/exchange/listings/latest", get(exchange_listings_latest))
        .with_state(st);

    let port = 8088u16;
    let addr = std::net::SocketAddr::from(([0,0,0,0], port));
    println!("indexer on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app).await.unwrap();
}


async fn domain_verifications_latest(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT block_number, tx_hash, outlet_id, domain, proof_type, proof_hash, verifier, inserted_at FROM outlet_domain_verifications ORDER BY block_number DESC LIMIT 300")
        .fetch_all(&st.db).await.unwrap_or_default();
    let out = rows.into_iter().map(|r| serde_json::json!({
        "block_number": r.get::<i64,_>("block_number"),
        "tx_hash": r.get::<String,_>("tx_hash"),
        "outlet_id": r.get::<String,_>("outlet_id"),
        "domain": r.get::<String,_>("domain"),
        "official_url": format!("https://{}", r.get::<String,_>("domain")),
        "proof_type": r.get::<i64,_>("proof_type"),
        "proof_hash": r.get::<String,_>("proof_hash"),
        "verifier": r.get::<String,_>("verifier"),
        "inserted_at": r.get::<String,_>("inserted_at"),
            "token_test": {
              "tx_hash": r.try_get::<String,_>("tx_hash").ok(),
              "symbol": r.try_get::<String,_>("symbol").ok(),
              "status": r.try_get::<i64,_>("status").ok(),
              "tested_at": r.try_get::<String,_>("tested_at").ok()
            },
    })).collect();
    Json(out)
}


#[derive(Deserialize)]
struct TokenTestReq {
    token: String,
    outlet_id: String,
    tx_hash: String,
    symbol: String,
    status: i64,
}

async fn token_test_write(State(st): State<AppState>, Json(req): Json<TokenTestReq>) -> Json<serde_json::Value> {
    let _ = sqlx::query("INSERT INTO token_tests (token, outlet_id, tx_hash, symbol, status, tested_at) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(&req.token).bind(&req.outlet_id).bind(&req.tx_hash).bind(&req.symbol).bind(req.status).bind(now_iso())
        .execute(&st.db).await;
    Json(serde_json::json!({"ok": true}))
}

async fn token_tests_latest(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT token, outlet_id, tx_hash, symbol, status, tested_at FROM token_tests ORDER BY tested_at DESC LIMIT 300")
        .fetch_all(&st.db).await.unwrap_or_default();
    let out = rows.into_iter().map(|r| serde_json::json!({
        "token": r.get::<String,_>("token"),
        "outlet_id": r.get::<String,_>("outlet_id"),
        "tx_hash": r.get::<String,_>("tx_hash"),
        "symbol": r.get::<String,_>("symbol"),
        "status": r.get::<i64,_>("status"),
        "tested_at": r.get::<String,_>("tested_at"),
    })).collect();
    Json(out)
}


#[derive(Deserialize)]
struct ApprovedWriteReq {
    proposal_id: i64,
    config_key: String,
    config_value: String,
    passed: i64,
    auto_applied: i64,
    reason: String,
}

async fn approved_write(State(st): State<AppState>, Json(req): Json<ApprovedWriteReq>) -> Json<serde_json::Value> {
    let _ = sqlx::query("INSERT INTO approved_updates (proposal_id, config_key, config_value, passed, auto_applied, reason, recorded_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(req.proposal_id).bind(&req.config_key).bind(&req.config_value).bind(req.passed).bind(req.auto_applied).bind(&req.reason).bind(now_iso())
        .execute(&st.db).await;
    Json(serde_json::json!({"ok": true}))
}

async fn approved_latest(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT proposal_id, config_key, config_value, passed, auto_applied, reason, recorded_at FROM approved_updates ORDER BY recorded_at DESC LIMIT 500")
        .fetch_all(&st.db).await.unwrap_or_default();
    let out = rows.into_iter().map(|r| serde_json::json!({
        "proposal_id": r.get::<i64,_>("proposal_id"),
        "config_key": r.get::<String,_>("config_key"),
        "config_value": r.get::<String,_>("config_value"),
        "passed": r.get::<i64,_>("passed"),
        "auto_applied": r.get::<i64,_>("auto_applied"),
        "reason": r.get::<String,_>("reason"),
        "recorded_at": r.get::<String,_>("recorded_at"),
    })).collect();
    Json(out)
}

#[derive(Deserialize)]
struct BatchWriteReq {
    batch_id: String,
    title: String,
    window_start: String,
    window_end: String,
    status: String,
    notes: String,
}

async fn batch_write(State(st): State<AppState>, Json(req): Json<BatchWriteReq>) -> Json<serde_json::Value> {
    let _ = sqlx::query("INSERT INTO release_batches (batch_id, title, window_start, window_end, status, notes, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(&req.batch_id).bind(&req.title).bind(&req.window_start).bind(&req.window_end).bind(&req.status).bind(&req.notes).bind(now_iso())
        .execute(&st.db).await;
    Json(serde_json::json!({"ok": true}))
}

async fn batch_latest(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT batch_id, title, window_start, window_end, status, notes, created_at FROM release_batches ORDER BY created_at DESC LIMIT 100")
        .fetch_all(&st.db).await.unwrap_or_default();
    let out = rows.into_iter().map(|r| serde_json::json!({
        "batch_id": r.get::<String,_>("batch_id"),
        "title": r.get::<String,_>("title"),
        "window_start": r.get::<String,_>("window_start"),
        "window_end": r.get::<String,_>("window_end"),
        "status": r.get::<String,_>("status"),
        "notes": r.get::<String,_>("notes"),
        "created_at": r.get::<String,_>("created_at"),
    })).collect();
    Json(out)
}


use ethers::prelude::*;
use ethers::types::{Filter, H256, U256, Address, Log};
use ethers::core::abi::{AbiDecode, RawLog, Token};
use std::sync::Arc;

fn read_state_string(path: &str) -> Option<String> {
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

fn h256(sig: &str) -> H256 {
    H256::from(ethers::utils::keccak256(sig.as_bytes()))
}

// Event signatures
// ProposalCreated(uint256,address,string,bytes32,int256,uint256,uint256,uint256)
const SIG_PROPOSAL_CREATED: &str = "ProposalCreated(uint256,address,string,bytes32,int256,uint256,uint256,uint256)";
// ProposalFinalized(uint256,bool,uint256,uint256,string,uint256,bool,uint256)
const SIG_BATCH_QUEUED: &str = "BatchQueued(bytes32,uint256,bytes32,int256,address,uint256)";
const SIG_GRANT_EXECUTED: &str = "GrantExecuted(uint256,address,uint256)";
const SIG_VOTE_FEE_CHARGED: &str = "VoteFeeCharged(address,uint256)";
const SIG_PROPOSAL_FINALIZED: &str = "ProposalFinalized(uint256,bool,uint256,uint256,string,uint256,bool,uint256)";

async fn governance_ingest_loop(st: AppState) {
    let gov_addr = read_state_string("/state/press_governance_address.txt");
    let rpc = std::env::var("RPC_URL").unwrap_or_else(|_| "http://press-rpc:8545".into());
    if gov_addr.is_none() {
        eprintln!("governance_ingest_loop: missing /state/press_governance_address.txt (skipping)");
        return;
    }
    let gov: Address = gov_addr.unwrap().parse().unwrap_or(Address::zero());
    if gov == Address::zero() { eprintln!("governance_ingest_loop: invalid governance address"); return; }

    let provider = Provider::<Http>::try_from(rpc).expect("provider");
    let provider = Arc::new(provider);

    let mut from_block: U64 = U64::from(0u64);
    let last_path = "/state/indexer_governance_lastblock.txt";

    if let Some(s) = read_state_string(last_path) {
        if let Ok(n) = s.parse::<u64>() { from_block = U64::from(n); }
    } else {
        // start near head
        if let Ok(head) = provider.get_block_number().await {
            from_block = head.saturating_sub(U64::from(2000u64));
        }
    }

    let topic_created = h256(SIG_PROPOSAL_CREATED);
    let topic_final = h256(SIG_PROPOSAL_FINALIZED);

    loop {
        let head = match provider.get_block_number().await {
            Ok(h) => h,
            Err(e) => { eprintln!("governance head error: {e}"); tokio::time::sleep(std::time::Duration::from_secs(3)).await; continue; }
        };
        let to_block = head;
        if to_block <= from_block {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            continue;
        }

        let f = Filter::new()
            .address(gov)
            .from_block(from_block)
            .to_block(to_block)
            .topic0(ValueOrArray::Array(vec![topic_created, topic_final]));

        let logs = match provider.get_logs(&f).await {
            Ok(l) => l,
            Err(e) => { eprintln!("governance get_logs error: {e}"); tokio::time::sleep(std::time::Duration::from_secs(3)).await; continue; }
        };

        for lg in logs {
            let t0 = lg.topics.get(0).cloned().unwrap_or_default();
            if t0 == topic_grant { let _ = handle_grant_executed(&st, &lg).await; }
            else if t0 == topic_votefee { let _ = handle_vote_fee(&st, &lg).await; }
            else if t0 == topic_created {
                let _ = handle_proposal_created(&st, &lg).await;
            } else if t0 == topic_final {
                let _ = handle_proposal_finalized(&st, &lg).await;
            }
        }

        from_block = to_block + U64::from(1u64);
        let _ = std::fs::write(last_path, format!("{}", from_block.as_u64()));
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

async fn handle_proposal_created(st: &AppState, lg: &Log) -> Result<(), String> {
    // topics: [sig, id, proposer]
    if lg.topics.len() < 3 { return Ok(()); }
    let proposal_id = U256::from_big_endian(lg.topics[1].as_bytes()).as_u64() as i64;
    let proposer = Address::from_slice(&lg.topics[2].as_bytes()[12..]).to_string();

    // data: title (string), configKey (bytes32), configValue (int256), feePaid (uint256), createdAt (uint256), endsAt (uint256)
    // We decode as tokens.
    let raw = RawLog{ topics: lg.topics.clone(), data: lg.data.to_vec() };
    // ABI for non-indexed: string, bytes32, int256, uint256, uint256, uint256
    let tokens = ethers::core::abi::decode(
        &[
            ParamType::String,
            ParamType::FixedBytes(32),
            ParamType::Int(256),
            ParamType::Uint(256),
            ParamType::Uint(256),
            ParamType::Uint(256)
        ],
        &raw.data
    ).map_err(|e| e.to_string())?;

    let title = tokens[0].clone().into_string().unwrap_or_default();
    let key_bytes = tokens[1].clone().into_fixed_bytes().unwrap_or_default();
    let config_key = format!("0x{}", hex::encode(key_bytes));
    let cfg_val = tokens[2].clone().into_int().unwrap_or_default();
    let config_value = cfg_val.to_string();

    let fee_paid = tokens[3].clone().into_uint().unwrap_or_default().to_string();
    let created_at = tokens[4].clone().into_uint().unwrap_or_default().to_string();
    let ends_at = tokens[5].clone().into_uint().unwrap_or_default().to_string();

    let _ = sqlx::query("INSERT OR REPLACE INTO governance_proposals (proposal_id, proposer, title, config_key, config_value, fee_paid, created_at, ends_at) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(proposal_id).bind(&proposer).bind(&title).bind(&config_key).bind(&config_value).bind(&fee_paid).bind(&created_at).bind(&ends_at)
        .execute(&st.db).await.map_err(|e| e.to_string())?;
let presets = load_presets();
if passed && !auto_applied {
    let is_batch = key_in_list(&presets, "release_batch_variables", &config_key);
    if is_batch {
        let (bid, ws, we) = month_batch_id();
        let title = format!("Monthly Release Batch {}", bid);
        ensure_release_batch(st, &bid, &title, &ws, &we).await;
        add_batch_item(st, &bid, proposal_id, &config_key, &config_value).await;
    }
}

    Ok(())
}

async fn handle_proposal_finalized(st: &AppState, lg: &Log) -> Result<(), String> {
    // topics: [sig, id]
    if lg.topics.len() < 2 { return Ok(()); }
    let proposal_id = U256::from_big_endian(lg.topics[1].as_bytes()).as_u64() as i64;

    // data: passed(bool), yesVotes(uint256), noVotes(uint256), reason(string), finalizedAt(uint256), autoApplied(bool), refundPaid(uint256)
    let raw = RawLog{ topics: lg.topics.clone(), data: lg.data.to_vec() };
    let tokens = ethers::core::abi::decode(
        &[
            ParamType::Bool,
            ParamType::Uint(256),
            ParamType::Uint(256),
            ParamType::String,
            ParamType::Uint(256),
            ParamType::Bool,
            ParamType::Uint(256)
        ],
        &raw.data
    ).map_err(|e| e.to_string())?;

    let passed = tokens[0].clone().into_bool().unwrap_or(false);
    let reason = tokens[3].clone().into_string().unwrap_or_default();
    let auto_applied = tokens[5].clone().into_bool().unwrap_or(false);

    // Lookup proposal details
    let row = sqlx::query("SELECT config_key, config_value FROM governance_proposals WHERE proposal_id=$1")
        .bind(proposal_id).fetch_optional(&st.db).await.map_err(|e| e.to_string())?;
    let (config_key, config_value) = if let Some(r)=row {
        (r.get::<String,_>("config_key"), r.get::<String,_>("config_value"))
    } else {
        ("0x".to_string(), "0".to_string())
    };

    let _ = sqlx::query("INSERT INTO approved_updates (proposal_id, config_key, config_value, passed, auto_applied, reason, recorded_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(proposal_id)
        .bind(&config_key)
        .bind(&config_value)
        .bind(if passed {1i64} else {0i64})
        .bind(if auto_applied {1i64} else {0i64})
        .bind(&reason)
        .bind(now_iso())
        .execute(&st.db).await.map_err(|e| e.to_string())?;
    Ok(())
}


fn load_presets() -> serde_json::Value {
    let s = std::fs::read_to_string("/state/proposal_presets.json")
        .or_else(|_| std::fs::read_to_string("config/proposal_presets.json"))
        .unwrap_or_else(|_| "{}".into());
    serde_json::from_str(&s).unwrap_or(serde_json::json!({}))
}

fn key_in_list(presets: &serde_json::Value, list: &str, key_hex: &str) -> bool {
    let arr = presets.get(list).and_then(|v| v.as_array()).cloned().unwrap_or_default();
    // preset keys are human strings, but on-chain stored config_key is keccak hash hex
    // we match by hashing the preset key string and comparing.
    for it in arr {
        if let Some(k) = it.get("key").and_then(|v| v.as_str()) {
            let h = ethers::utils::keccak256(k.as_bytes());
            let hx = format!("0x{}", hex::encode(h));
            if hx.eq_ignore_ascii_case(key_hex) { return true; }
        }
    }
    false
}

fn month_batch_id() -> (String,String,String) {
    let now = chrono::Utc::now();
    let bid = format!("{}-{:02}", now.year(), now.month());
    let start = chrono::Utc.with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0).unwrap();
    let next = if now.month()==12 {
        chrono::Utc.with_ymd_and_hms(now.year()+1, 1, 1, 0, 0, 0).unwrap()
    } else {
        chrono::Utc.with_ymd_and_hms(now.year(), now.month()+1, 1, 0, 0, 0).unwrap()
    };
    let end = next - chrono::Duration::seconds(1);
    (bid, start.to_rfc3339(), end.to_rfc3339())
}

async fn ensure_release_batch(st: &AppState, batch_id: &str, title: &str, ws: &str, we: &str) {
    let _ = sqlx::query("INSERT INTO release_batches (batch_id, title, window_start, window_end, status, notes, created_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(batch_id).bind(title).bind(ws).bind(we).bind("planned").bind("Auto-created by indexer").bind(now_iso())
        .execute(&st.db).await;
}

async fn add_batch_item(st: &AppState, batch_id: &str, proposal_id: i64, config_key: &str, config_value: &str) {
    let _ = sqlx::query("INSERT OR IGNORE INTO release_batch_items (batch_id, proposal_id, config_key, config_value, status, created_at) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(batch_id).bind(proposal_id).bind(config_key).bind(config_value).bind("queued").bind(now_iso())
        .execute(&st.db).await;
}


async fn batch_items_latest(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT batch_id, proposal_id, config_key, config_value, status, created_at FROM release_batch_items ORDER BY created_at DESC LIMIT 500")
        .fetch_all(&st.db).await.unwrap_or_default();
    let out = rows.into_iter().map(|r| serde_json::json!({
        "batch_id": r.get::<String,_>("batch_id"),
        "proposal_id": r.get::<i64,_>("proposal_id"),
        "config_key": r.get::<String,_>("config_key"),
        "config_value": r.get::<String,_>("config_value"),
        "status": r.get::<String,_>("status"),
        "created_at": r.get::<String,_>("created_at"),
    })).collect();
    Json(out)
}


async fn upgrade_queue_ingest_loop(st: AppState) {
    let q_addr = read_state_string("/state/press_upgrade_queue_address.txt");
    let rpc = std::env::var("RPC_URL").unwrap_or_else(|_| "http://press-rpc:8545".into());
    if q_addr.is_none() {
        eprintln!("upgrade_queue_ingest_loop: missing /state/press_upgrade_queue_address.txt (skipping)");
        return;
    }
    let q: Address = q_addr.unwrap().parse().unwrap_or(Address::zero());
    if q == Address::zero() { eprintln!("upgrade_queue_ingest_loop: invalid queue address"); return; }

    let provider = Provider::<Http>::try_from(rpc).expect("provider");
    let provider = Arc::new(provider);

    let mut from_block: U64 = U64::from(0u64);
    let last_path = "/state/indexer_upgradequeue_lastblock.txt";

    if let Some(s) = read_state_string(last_path) {
        if let Ok(n) = s.parse::<u64>() { from_block = U64::from(n); }
    } else {
        if let Ok(head) = provider.get_block_number().await {
            from_block = head.saturating_sub(U64::from(2000u64));
        }
    }

    let topic_batch = h256(SIG_BATCH_QUEUED);

    loop {
        let head = match provider.get_block_number().await {
            Ok(h) => h,
            Err(e) => { eprintln!("upgradequeue head error: {e}"); tokio::time::sleep(std::time::Duration::from_secs(3)).await; continue; }
        };
        let to_block = head;
        if to_block <= from_block {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            continue;
        }

        let f = Filter::new()
            .address(q)
            .from_block(from_block)
            .to_block(to_block)
            .topic0(topic_batch);

        let logs = match provider.get_logs(&f).await {
            Ok(l) => l,
            Err(e) => { eprintln!("upgradequeue get_logs error: {e}"); tokio::time::sleep(std::time::Duration::from_secs(3)).await; continue; }
        };

        for lg in logs {
            let _ = handle_batch_queued(&st, &lg).await;
        }

        from_block = to_block + U64::from(1u64);
        let _ = std::fs::write(last_path, format!("{}", from_block.as_u64()));
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

async fn handle_batch_queued(st: &AppState, lg: &Log) -> Result<(), String> {
    // indexed: batchId(bytes32), proposalId(uint256), configKey(bytes32)
    if lg.topics.len() < 4 { return Ok(()); }
    let batch_id_hex = format!("0x{}", hex::encode(lg.topics[1].as_bytes()));
    let proposal_id = U256::from_big_endian(lg.topics[2].as_bytes()).as_u64() as i64;
    let config_key = format!("0x{}", hex::encode(lg.topics[3].as_bytes()));
    // data: configValue(int256), queuedBy(address), queuedAt(uint256)
    let raw = RawLog{ topics: lg.topics.clone(), data: lg.data.to_vec() };
    let tokens = ethers::core::abi::decode(
        &[
            ParamType::Int(256),
            ParamType::Address,
            ParamType::Uint(256)
        ],
        &raw.data
    ).map_err(|e| e.to_string())?;

    let cfg_val = tokens[0].clone().into_int().unwrap_or_default().to_string();
    // ensure batch meta exists (month unknown from batchId; store minimal record)
    ensure_release_batch(st, &batch_id_hex, &format!("Release Batch {}", batch_id_hex), &now_iso(), &now_iso()).await;
    add_batch_item(st, &batch_id_hex, proposal_id, &config_key, &cfg_val).await;
    Ok(())
}


// Exchange Listing Registry ingestion
const SIG_LISTING_REQUESTED: &str = "ListingRequested(address,bytes32,uint256)";
const SIG_TEST_PASSED: &str = "TestTransactionPassed(address)";
const SIG_LISTING_FINALIZED: &str = "ListingFinalized(address,string,bytes32)";

async fn exchange_registry_ingest_loop(st: AppState) {
    let addr = read_state_string("/state/exchange_listing_registry_address.txt");
    let rpc = std::env::var("RPC_URL").unwrap_or_else(|_| "http://press-rpc:8545".into());
    if addr.is_none() { eprintln!("exchange_registry_ingest_loop: missing exchange_listing_registry_address (skip)"); return; }
    let reg: Address = addr.unwrap().parse().unwrap_or(Address::zero());
    if reg == Address::zero() { return; }
    let provider = Provider::<Http>::try_from(rpc).expect("provider");
    let provider = Arc::new(provider);

    let mut from_block: U64 = U64::from(0u64);
    let last_path = "/state/indexer_exchange_lastblock.txt";
    if let Some(s) = read_state_string(last_path) { if let Ok(n)=s.parse::<u64>(){ from_block=U64::from(n);} }
    else if let Ok(head)=provider.get_block_number().await { from_block = head.saturating_sub(U64::from(2000u64)); }

    let t_req = h256(SIG_LISTING_REQUESTED);
    let t_test = h256(SIG_TEST_PASSED);
    let t_fin = h256(SIG_LISTING_FINALIZED);

    loop {
        let head = match provider.get_block_number().await { Ok(h)=>h, Err(e)=>{eprintln!("exchange head error: {e}"); tokio::time::sleep(std::time::Duration::from_secs(3)).await; continue;} };
        let to_block = head;
        if to_block <= from_block { tokio::time::sleep(std::time::Duration::from_secs(3)).await; continue; }

        let f = Filter::new()
            .address(reg)
            .from_block(from_block)
            .to_block(to_block)
            .topic0(ValueOrArray::Array(vec![t_req, t_test, t_fin]));

        let logs = match provider.get_logs(&f).await { Ok(l)=>l, Err(e)=>{eprintln!("exchange get_logs error: {e}"); tokio::time::sleep(std::time::Duration::from_secs(3)).await; continue;} };

        for lg in logs {
            let t0 = lg.topics.get(0).cloned().unwrap_or_default();
            if t0 == t_req { let _ = handle_listing_requested(&st, &lg).await; }
            else if t0 == t_test { let _ = handle_test_passed(&st, &lg).await; }
            else if t0 == t_fin { let _ = handle_listing_finalized(&st, &lg).await; }
        }

        from_block = to_block + U64::from(1u64);
        let _ = std::fs::write(last_path, format!("{}", from_block.as_u64()));
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

async fn handle_listing_requested(st: &AppState, lg: &Log) -> Result<(), String> {
    // topics: [sig, outlet]
    if lg.topics.len() < 2 { return Ok(()); }
    let outlet = Address::from_slice(&lg.topics[1].as_bytes()[12..]).to_string();
    // data: tier(bytes32), feePaid(uint256)
    let raw = RawLog{ topics: lg.topics.clone(), data: lg.data.to_vec() };
    let toks = ethers::core::abi::decode(&[ParamType::FixedBytes(32), ParamType::Uint(256)], &raw.data).map_err(|e| e.to_string())?;
    let tier = format!("0x{}", hex::encode(toks[0].clone().into_fixed_bytes().unwrap_or_default()));
    let fee = toks[1].clone().into_uint().unwrap_or_default().to_string();
    // upsert
    let _ = sqlx::query("INSERT OR REPLACE INTO exchange_listings (outlet,tier,domain,fee_paid,test_passed,listed_at,updated_at) VALUES ($1,$2,$3,$4,$5,$6,$7)")
        .bind(&outlet).bind(&tier).bind("").bind(&fee).bind(0i64).bind("").bind(now_iso())
        .execute(&st.db).await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn handle_test_passed(st: &AppState, lg: &Log) -> Result<(), String> {
    if lg.topics.len() < 2 { return Ok(()); }
    let outlet = Address::from_slice(&lg.topics[1].as_bytes()[12..]).to_string();
    let _ = sqlx::query("UPDATE exchange_listings SET test_passed=1, updated_at=$2 WHERE outlet=$1")
        .bind(&outlet).bind(now_iso())
        .execute(&st.db).await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn handle_listing_finalized(st: &AppState, lg: &Log) -> Result<(), String> {
    if lg.topics.len() < 2 { return Ok(()); }
    let outlet = Address::from_slice(&lg.topics[1].as_bytes()[12..]).to_string();
    // data: domain(string), tier(bytes32)
    let raw = RawLog{ topics: lg.topics.clone(), data: lg.data.to_vec() };
    let toks = ethers::core::abi::decode(&[ParamType::String, ParamType::FixedBytes(32)], &raw.data).map_err(|e| e.to_string())?;
    let domain = toks[0].clone().into_string().unwrap_or_default();
    let tier = format!("0x{}", hex::encode(toks[1].clone().into_fixed_bytes().unwrap_or_default()));
    let _ = sqlx::query("UPDATE exchange_listings SET domain=$2, tier=$3, listed_at=$4, updated_at=$5 WHERE outlet=$1")
        .bind(&outlet).bind(&domain).bind(&tier).bind(now_iso()).bind(now_iso())
        .execute(&st.db).await.map_err(|e| e.to_string())?;
    Ok(())
}


async fn exchange_listings_latest(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT outlet, tier, domain, fee_paid, test_passed, listed_at, updated_at FROM exchange_listings ORDER BY updated_at DESC LIMIT 500")
        .fetch_all(&st.db).await.unwrap_or_default();
    let out = rows.into_iter().map(|r| serde_json::json!({
        "outlet": r.get::<String,_>("outlet"),
        "tier": r.get::<String,_>("tier"),
        "domain": r.get::<String,_>("domain"),
        "fee_paid": r.get::<String,_>("fee_paid"),
        "test_passed": r.get::<i64,_>("test_passed"),
        "listed_at": r.get::<String,_>("listed_at"),
        "updated_at": r.get::<String,_>("updated_at"),
    })).collect();
    Json(out)
}


async fn handle_vote_fee(st: &AppState, lg: &Log) -> Result<(), String> {
    // topics: [sig, voter]
    if lg.topics.len() < 2 { return Ok(()); }
    let voter = Address::from_slice(&lg.topics[1].as_bytes()[12..]).to_string();
    // data: amount(uint256)
    let raw = RawLog{ topics: lg.topics.clone(), data: lg.data.to_vec() };
    let toks = ethers::core::abi::decode(&[ParamType::Uint(256)], &raw.data).map_err(|e| e.to_string())?;
    let amt = toks[0].clone().into_uint().unwrap_or_default().to_string();
    let txh = lg.transaction_hash.map(|h| format!("0x{}", hex::encode(h.as_bytes()))).unwrap_or_else(|| "".into());
    let bn = lg.block_number.map(|b| b.as_u64() as i64).unwrap_or(0);
    let _ = sqlx::query("INSERT OR REPLACE INTO governance_vote_fees (tx_hash,voter,amount,block_num,created_at) VALUES ($1,$2,$3,$4,$5)")
        .bind(&txh).bind(&voter).bind(&amt).bind(bn).bind(now_iso())
        .execute(&st.db).await.map_err(|e| e.to_string())?;
    Ok(())
}


async fn vote_fees_latest(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT tx_hash, voter, amount, block_num, created_at FROM governance_vote_fees ORDER BY block_num DESC LIMIT 1000")
        .fetch_all(&st.db).await.unwrap_or_default();
    let out = rows.into_iter().map(|r| serde_json::json!({
        "tx_hash": r.get::<String,_>("tx_hash"),
        "voter": r.get::<String,_>("voter"),
        "amount": r.get::<String,_>("amount"),
        "block_num": r.get::<i64,_>("block_num"),
        "created_at": r.get::<String,_>("created_at")
    })).collect();
    Json(out)
}


async fn handle_grant_executed(st: &AppState, lg: &Log) -> Result<(), String> {
    // data: id(uint256), recipient(address), amount(uint256)
    let raw = RawLog{ topics: lg.topics.clone(), data: lg.data.to_vec() };
    let toks = ethers::core::abi::decode(&[ParamType::Uint(256), ParamType::Address, ParamType::Uint(256)], &raw.data).map_err(|e| e.to_string())?;
    let id = toks[0].clone().into_uint().unwrap_or_default().as_u64() as i64;
    let rec = toks[1].clone().into_address().unwrap_or_default().to_string();
    let amt = toks[2].clone().into_uint().unwrap_or_default().to_string();
    let txh = lg.transaction_hash.map(|h| format!("0x{}", hex::encode(h.as_bytes()))).unwrap_or_else(|| "".into());
    let bn = lg.block_number.map(|b| b.as_u64() as i64).unwrap_or(0);
    let _ = sqlx::query("INSERT OR REPLACE INTO governance_grants (tx_hash,proposal_id,recipient,amount,block_num,created_at) VALUES ($1,$2,$3,$4,$5,$6)")
        .bind(&txh).bind(id).bind(&rec).bind(&amt).bind(bn).bind(now_iso())
        .execute(&st.db).await.map_err(|e| e.to_string())?;
    Ok(())
}


async fn grants_latest(State(st): State<AppState>) -> Json<Vec<serde_json::Value>> {
    let rows = sqlx::query("SELECT tx_hash, proposal_id, recipient, amount, block_num, created_at FROM governance_grants ORDER BY block_num DESC LIMIT 500")
        .fetch_all(&st.db).await.unwrap_or_default();
    let out = rows.into_iter().map(|r| serde_json::json!({
        "tx_hash": r.get::<String,_>("tx_hash"),
        "proposal_id": r.get::<i64,_>("proposal_id"),
        "recipient": r.get::<String,_>("recipient"),
        "amount": r.get::<String,_>("amount"),
        "block_num": r.get::<i64,_>("block_num"),
        "created_at": r.get::<String,_>("created_at")
    })).collect();
    Json(out)
}


use axum::extract::{Query, Path};

#[derive(Deserialize)]
struct IdxSearch { q: String }

async fn search(Query(q): Query<IdxSearch>) -> Json<serde_json::Value> {
    // TODO: query real index tables. Placeholder returns empty.
    Json(serde_json::json!({"ok": true, "query": q.q, "items": []}))
}

#[derive(Deserialize)]
struct IdxFeedQ { after: Option<i64>, outlet: Option<String>, article_id: Option<String>, kind: Option<String>, severity: Option<i64> }

async fn feed(Path(feed): Path<String>, Query(q): Query<IdxFeedQ>) -> Json<serde_json::Value> {
    let after = q.after.unwrap_or(0);
    // Feeds: recent_articles, pending_votes, proposals, oracle_flags
    Json(serde_json::json!({"ok": true, "feed": feed, "after": after, "items": []}))
}


use serde::{Serialize, Deserialize};
use std::sync::Mutex;

// --- RR85: Article lifecycle + Oracle flags ---
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ArticleEventType {
    Submitted,
    VotingStarted,
    VotingEnded,
    Approved,
    Rejected,
    Flagged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ArticleEvent {
    ts: i64,
    article_id: String,
    outlet: String,
    event: ArticleEventType,
    metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OracleFlag {
    ts: i64,
    article_id: String,
    severity: u8, // 1-5
    kind: String, // copyright, conflict, similarity
    source: String, // ai, human, external
    details: serde_json::Value,
}


lazy_static::lazy_static! {
    static ref EVENTS: Mutex<Vec<ArticleEvent>> = Mutex::new(Vec::new());
    static ref FLAGS: Mutex<Vec<OracleFlag>> = Mutex::new(Vec::new());
}

async fn emit_event(db: &SqlitePool, ev: ArticleEvent) {
    EVENTS.lock().unwrap().push(ev.clone());
    let meta = ev.metadata.to_string();
    let event_str = format!("{:?}", ev.event);
    let title = ev.metadata.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
    let url = ev.metadata.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());

    let canonical_text = req.metadata.get("canonical_text").and_then(|v| v.as_str()).map(|s| s.to_string());
    let content_hash = req.metadata.get("content_hash").and_then(|v| v.as_str()).map(|s| s.to_string());
    let _ = sqlx::query("INSERT INTO press_events(ts,article_id,outlet,event,title,url,canonical_text,content_hash,metadata) VALUES(?,?,?,?,?,?,?,?,?)")
        .bind(ev.ts)
        .bind(ev.article_id)
        .bind(ev.outlet)
        .bind(event_str)
        .bind(title)
        .bind(url)
        .bind(meta)
        .execute(db)
        .await;
}

async fn emit_flag(db: &SqlitePool, flag: OracleFlag) {
    FLAGS.lock().unwrap().push(flag.clone());
    let det = flag.details.to_string();
    let title = flag.details.get("title").and_then(|v| v.as_str()).map(|s| s.to_string());
    let url = flag.details.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());

    let _ = sqlx::query("INSERT INTO oracle_flags(ts,article_id,severity,kind,source,title,url,details) VALUES(?,?,?,?,?,?,?,?)")
        .bind(flag.ts)
        .bind(flag.article_id)
        .bind(flag.severity as i64)
        .bind(flag.kind)
        .bind(flag.source)
        .bind(title)
        .bind(url)
        .bind(det)
        .execute(db)
        .await;
}

async fn init_schema(db: &SqlitePool) -> anyhow::Result<()> {
    // Core event stream
    sqlx::query(r#"
    CREATE TABLE IF NOT EXISTS press_events (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        ts INTEGER NOT NULL,
        article_id TEXT NOT NULL,
        outlet TEXT NOT NULL,
        event TEXT NOT NULL,
        title TEXT,
        url TEXT,
        canonical_text TEXT,
        content_hash TEXT,
        metadata TEXT NOT NULL
    );
    "#).execute(db).await?;

// Add new columns if missing (SQLite safe on fresh DB; on existing, ignore failures)
let _ = sqlx::query("ALTER TABLE press_events ADD COLUMN canonical_text TEXT").execute(db).await;
let _ = sqlx::query("ALTER TABLE press_events ADD COLUMN content_hash TEXT").execute(db).await;


    // Oracle flags
    sqlx::query(r#"
    CREATE TABLE IF NOT EXISTS oracle_flags (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        ts INTEGER NOT NULL,
        article_id TEXT NOT NULL,
        severity INTEGER NOT NULL,
        kind TEXT NOT NULL,
        source TEXT NOT NULL,
        title TEXT,
        url TEXT,
        details TEXT NOT NULL
    );
    "#).execute(db).await?;

    // Index-friendly lookup
    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_events_ts ON press_events(ts);"#).execute(db).await?;
    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_flags_ts ON oracle_flags(ts);"#).execute(db).await?;
    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_events_article ON press_events(article_id);"#).execute(db).await?;
    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_flags_article ON oracle_flags(article_id);"#).execute(db).await?;

    // Uptime heartbeats (on-chain -> indexed)
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS heartbeats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            block_number BIGINT NOT NULL,
            tx_hash TEXT NOT NULL,
            service TEXT NOT NULL,
            ts INTEGER NOT NULL,
            status INTEGER NOT NULL,
            extra TEXT NOT NULL
        );
    "#).execute(db).await?;
    sqlx::query(r#"CREATE INDEX IF NOT EXISTS idx_heartbeats_service_ts ON heartbeats(service, ts);"#).execute(db).await?;

    Ok(())
}


#[derive(Deserialize)]
struct PostEventReq {
    ts: Option<i64>,
    article_id: String,
    outlet: String,
    event: String, // Submitted/VotingStarted/Approved/...
    title: Option<String>,
    url: Option<String>,
    metadata: serde_json::Value,
}

async fn post_event(State(state): State<Arc<AppState>>, Json(req): Json<PostEventReq>) -> Json<serde_json::Value> {
    let ts = req.ts.unwrap_or_else(|| chrono::Utc::now().timestamp());
    let meta = req.metadata.to_string();
    let canonical_text = req.metadata.get("canonical_text").and_then(|v| v.as_str()).map(|s| s.to_string());
    let content_hash = req.metadata.get("content_hash").and_then(|v| v.as_str()).map(|s| s.to_string());
    let _ = sqlx::query("INSERT INTO press_events(ts,article_id,outlet,event,title,url,canonical_text,content_hash,metadata) VALUES(?,?,?,?,?,?,?,?,?)")
        .bind(ts)
        .bind(&req.article_id)
        .bind(&req.outlet)
        .bind(&req.event)
        .bind(&req.title)
        .bind(&req.url)
        .bind(&canonical_text)
        .bind(&content_hash)
        .bind(&meta)
        .execute(&state.db).await;
    Json(serde_json::json!({"ok": true}))
}

#[derive(Deserialize)]
struct PostFlagReq {
    ts: Option<i64>,
    article_id: String,
    severity: i64,
    kind: String,
    source: String,
    title: Option<String>,
    url: Option<String>,
    details: serde_json::Value,
}

async fn post_flag(State(state): State<Arc<AppState>>, Json(req): Json<PostFlagReq>) -> Json<serde_json::Value> {
    let ts = req.ts.unwrap_or_else(|| chrono::Utc::now().timestamp());
    let det = req.details.to_string();
    let _ = sqlx::query("INSERT INTO oracle_flags(ts,article_id,severity,kind,source,title,url,details) VALUES(?,?,?,?,?,?,?,?)")
        .bind(ts)
        .bind(&req.article_id)
        .bind(req.severity)
        .bind(&req.kind)
        .bind(&req.source)
        .bind(&req.title)
        .bind(&req.url)
        .bind(&det)
        .execute(&state.db).await;
    Json(serde_json::json!({"ok": true}))
}
