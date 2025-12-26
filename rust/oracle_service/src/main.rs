use axum::{routing::{get, post}, Json, Router, extract::ConnectInfo};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};

#[derive(Clone)]
struct Cfg {
    indexer_api: String,
    query_api: String,
    openai_key: Option<String>,
    shared_secret: Option<String>,
    model: String,
    state_dir: String,
    daily_max_calls: u32,
    daily_max_tokens: u32,
}

#[derive(Deserialize)]
struct AnalyzeReq {
    article_id: String,
    outlet: String,
    title: Option<String>,
    url: Option<String>,
    content: Option<String>, // optional raw text
}

#[derive(Serialize)]
struct AnalyzeResp { ok: bool, flags: Vec<serde_json::Value> }

fn require_sig(cfg: &Cfg, sig: Option<String>, body: &str) -> bool {
    // HMAC-SHA256 of body using shared secret, base64url
    let secret = match cfg.shared_secret.as_ref() {
        Some(s) if !s.is_empty() => s,
        _ => return true, // if unset, allow (for local/dev); production should set it
    };
    let sig = match sig { Some(s)=>s, None=>return false };
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(body.as_bytes());
    let res = mac.finalize().into_bytes();
    let exp = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(res);
    sig == exp
}

fn html_to_text(html: &str) -> String {
    // very lightweight: strip tags + unescape entities
    let re_tags = regex::Regex::new(r"(?s)<script.*?</script>|<style.*?</style>|<[^>]+>").unwrap();
    let no = re_tags.replace_all(html, " ");
    let un = html_escape::decode_html_entities(&no);
    let t = un.to_string();
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn fingerprint(text: &str) -> String {
    let hash = sha2::Sha256::digest(text.as_bytes());
    format!("{:x}", hash)
}

async fn health() -> Json<serde_json::Value> { Json(serde_json::json!({"ok": true})) }

// RR87: local heuristic oracle (no external dependencies required).
// - Similarity: sha256(content) prefix + simple token overlap score (placeholder)
// - Conflict: if content contains "BREAKING" and "UPDATE" with contradictions (placeholder)
// - Copyright: if content includes obvious markers like "©" + long quote block (placeholder)
// Optional: OpenAI integration can be enabled later by providing OPENAI_API_KEY; this service will not log it.
async fn analyze(ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>, Json(req): Json<AnalyzeReq>, axum::extract::State(cfg): axum::extract::State<Cfg>) -> Json<AnalyzeResp> {
    if !allow_key(&format!("ip:{}", addr.ip()), 30, 60) { return Json(AnalyzeResp{ok:false, flags: vec![]}); }
    if !allow_key(&format!("article:{}", req.article_id), 10, 300) { return Json(AnalyzeResp{ok:false, flags: vec![]}); }
    let mut flags = Vec::new();
    let content = req.content.clone().unwrap_or_default();
    let canonical = content.clone();
    let content_hash = if canonical.is_empty() { String::new() } else { fingerprint(&canonical) };

    // Heuristic: copyright marker + long quote
    if content.contains("©") || content.contains("All rights reserved") {
        flags.push(serde_json::json!({
            "severity": 3,
            "kind": "copyright",
            "source": "ai",
            "details": {"reason":"copyright markers detected"}
        }));
    }
    if content.lines().any(|l| l.trim().starts_with(">")) && content.len() > 1500 {
        flags.push(serde_json::json!({
            "severity": 4,
            "kind": "copyright",
            "source": "ai",
            "details": {"reason":"large quoted block detected"}
        }));
    }

    // Heuristic: conflict keywords (placeholder)
    if content.to_lowercase().contains("conflicting reports") || content.to_lowercase().contains("unconfirmed") {
        flags.push(serde_json::json!({
            "severity": 2,
            "kind": "conflict",
            "source": "ai",
            "details": {"reason":"conflict/uncertainty language detected"}
        }));
    }

    // Similarity: compare fingerprint and token overlap against indexed corpus
    if !canonical.is_empty() {
        // Query recent corpus (by title hash or keywords)
        let q = req.title.clone().unwrap_or_else(|| "press".into());
        let url = format!("{}/search?q={}", cfg.indexer_api, urlencoding::encode(&q));
        let mut best = 0.0f64;
        if let Ok(r) = reqwest::Client::new().get(url).send().await {
            if let Ok(j) = r.json::<serde_json::Value>().await {
                if let Some(items) = j.get("items").and_then(|v| v.as_array()) {
                    for it in items.iter().take(25) {
                        if let Some(mid) = it.get("article_id").and_then(|v| v.as_str()) {
                            // fetch canonical text via feed filter
                            let f = format!("{}/feed/recent_articles?after=0&article_id={}", cfg.indexer_api, urlencoding::encode(mid));
                            if let Ok(fr) = reqwest::Client::new().get(f).send().await {
                                if let Ok(fj) = fr.json::<serde_json::Value>().await {
                                    if let Some(arr) = fj.get("items").and_then(|v| v.as_array()) {
                                        if let Some(first) = arr.first() {
                                            if let Some(meta) = first.get("metadata").and_then(|v| v.as_str()) {
                                                // metadata is string JSON in indexer feed
                                                if let Ok(mj) = serde_json::from_str::<serde_json::Value>(meta) {
                                                    if let Some(ct) = mj.get("canonical_text").and_then(|v| v.as_str()) {
                                                        // token overlap score
                                                        let a: std::collections::HashSet<&str> = canonical.split_whitespace().take(800).collect();
                                                        let b: std::collections::HashSet<&str> = ct.split_whitespace().take(800).collect();
                                                        let inter = a.intersection(&b).count() as f64;
                                                        let denom = (a.len().max(1) as f64);
                                                        let score = inter / denom;
                                                        if score > best { best = score; }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if best > 0.35 {
            flags.push(serde_json::json!({
                "severity": 4,
                "kind": "similarity",
                "source": "ai",
                "details": {"score": best, "reason":"high token overlap vs indexed corpus"}
            }));
        } else if best > 0.20 {
            flags.push(serde_json::json!({
                "severity": 2,
                "kind": "similarity",
                "source": "ai",
                "details": {"score": best, "reason":"moderate similarity vs indexed corpus"}
            }));
        }
    }

    // Persist canonical text hash for future comparisons

    if !canonical.is_empty() {
        flags.push(serde_json::json!({
            "severity": 1,
            "kind": "similarity",
            "source": "ai",
            "details": {"content_hash": content_hash}
        }));
    }

    // Persist article event (oracle processed) with canonical_text + content_hash
    let http = reqwest::Client::builder().timeout(Duration::from_secs(5)).build().unwrap();
    let _ = http.post(format!("{}/events", cfg.indexer_api))
        .json(&serde_json::json!({
            "article_id": req.article_id,
            "outlet": req.outlet,
            "event": "OracleProcessed",
            "title": req.title,
            "url": req.url,
            "metadata": {
                "canonical_text": canonical,
                "content_hash": content_hash,
                "oracle": "PressOracle"
            }
        }))
        .send().await;

    // Persist flags to indexer
    let http = reqwest::Client::builder().timeout(Duration::from_secs(5)).build().unwrap();
    for f in flags.iter() {
        let sev = f.get("severity").and_then(|v| v.as_i64()).unwrap_or(1);
        let kind = f.get("kind").and_then(|v| v.as_str()).unwrap_or("similarity");
        let src = f.get("source").and_then(|v| v.as_str()).unwrap_or("ai");
        let details = f.get("details").cloned().unwrap_or(serde_json::json!({}));
        let _ = http.post(format!("{}/oracle/flag", cfg.indexer_api))
            .json(&serde_json::json!({
                "article_id": req.article_id,
                "severity": sev,
                "kind": kind,
                "source": src,
                "title": req.title,
                "url": req.url,
                "details": details
            }))
            .send().await;
    }

    Json(AnalyzeResp{ ok: true, flags })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cfg = Cfg{
        indexer_api: std::env::var("PRESS_INDEXER_API").unwrap_or_else(|_| "http://press-indexer:8786".into()),
        query_api: std::env::var("PRESS_QUERY_API").unwrap_or_else(|_| "http://query-api:8787".into()),
        openai_key: std::env::var("OPENAI_API_KEY").ok(),
        shared_secret: std::env::var("ORACLE_SHARED_SECRET").ok(),
        model: std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into()),
        state_dir: std::env::var("ORACLE_STATE_DIR").unwrap_or_else(|_| "/state".into()),
        daily_max_calls: std::env::var("ORACLE_DAILY_MAX_CALLS").ok().and_then(|v| v.parse().ok()).unwrap_or(50),
        daily_max_tokens: std::env::var("ORACLE_DAILY_MAX_TOKENS").ok().and_then(|v| v.parse().ok()).unwrap_or(20000),
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/oracle/analyze", post(analyze))
        .route("/api/oracle/ingest_url", post(ingest_url))
        .route("/api/oracle/analyze_sources", post(analyze_sources))
        .with_state(cfg);

    let addr: SocketAddr = "0.0.0.0:8790".parse().unwrap();
    println!("oracle_service listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await?;
    Ok(())
}


#[derive(Deserialize)]
struct IngestReq {
    article_id: String,
    outlet: String,
    url: String,
    title: Option<String>,
    // optional signed payload
    sig: Option<String>,
}

async fn ingest_url(
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    axum::extract::State(cfg): axum::extract::State<Cfg>,
    Json(req): Json<IngestReq>
) -> Json<serde_json::Value> {
    if !allow_key(&format!("ip:{}", addr.ip()), 30, 60) { return Json(serde_json::json!({"ok": false, "error":"rate_limited"})); }
    // body for signing (deterministic)
    let body = serde_json::json!({"article_id":req.article_id,"outlet":req.outlet,"url":req.url,"title":req.title}).to_string();
    if !require_sig(&cfg, req.sig.clone(), &body) {
        return Json(serde_json::json!({"ok": false, "error":"invalid signature"}));
    }
    // Fetch URL
    let http = reqwest::Client::builder().timeout(Duration::from_secs(8)).build().unwrap();
    let html = match http.get(&req.url).send().await.and_then(|r| r.text().await) {
        Ok(t)=>t, Err(_)=>String::new()
    };
    let text = html_to_text(&html);
    // Run analyze pipeline and persist flags/events
    let ar = AnalyzeReq{
        article_id: req.article_id,
        outlet: req.outlet,
        title: req.title,
        url: Some(req.url),
        content: Some(text),
    };
    let resp = analyze(Json(ar), axum::extract::State(cfg)).await;
    Json(serde_json::json!({"ok": true, "result": resp.0}))
}


use std::collections::HashMap;
use std::sync::Mutex as StdMutex;
lazy_static::lazy_static! {
    static ref RL: StdMutex<HashMap<String, (i64, u32)>> = StdMutex::new(HashMap::new());
}
fn allow_key(key: &str, limit: u32, window_sec: i64) -> bool {
    let now = chrono::Utc::now().timestamp();
    let mut m = RL.lock().unwrap();
    let e = m.entry(key.to_string()).or_insert((now, 0));
    if now - e.0 > window_sec { *e = (now, 0); }
    if e.1 >= limit { return false; }
    e.1 += 1;
    true
}


#[derive(Serialize, Deserialize, Default, Clone)]
struct BudgetState {
    date: String, // YYYY-MM-DD
    calls: u32,
    tokens: u32,
}

fn budget_path(cfg: &Cfg) -> std::path::PathBuf {
    let mut p = std::path::PathBuf::from(&cfg.state_dir);
    p.push("oracle_budget.json");
    p
}

fn load_budget(cfg: &Cfg) -> BudgetState {
    let p = budget_path(cfg);
    if let Ok(b) = std::fs::read_to_string(p) {
        if let Ok(s) = serde_json::from_str::<BudgetState>(&b) {
            return s;
        }
    }
    BudgetState::default()
}

fn save_budget(cfg: &Cfg, st: &BudgetState) {
    let p = budget_path(cfg);
    let _ = std::fs::create_dir_all(&cfg.state_dir);
    let _ = std::fs::write(p, serde_json::to_string_pretty(st).unwrap_or_default());
}

fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn allow_budget(cfg: &Cfg, add_calls: u32, add_tokens: u32) -> bool {
    let mut st = load_budget(cfg);
    let td = today();
    if st.date != td {
        st.date = td;
        st.calls = 0;
        st.tokens = 0;
    }
    if st.calls + add_calls > cfg.daily_max_calls { return false; }
    if st.tokens + add_tokens > cfg.daily_max_tokens { return false; }
    st.calls += add_calls;
    st.tokens += add_tokens;
    save_budget(cfg, &st);
    true
}

fn redact_secrets(s: &str) -> String {
    // basic redactions: OpenAI keys, hex private keys, JWTs
    let mut out = s.to_string();
    let re_openai = regex::Regex::new(r"sk-[A-Za-z0-9]{20,}").unwrap();
    out = re_openai.replace_all(&out, "sk-REDACTED").to_string();
    let re_hex = regex::Regex::new(r"0x[a-fA-F0-9]{64}").unwrap();
    out = re_hex.replace_all(&out, "0xREDACTEDPRIVATEKEY").to_string();
    let re_jwt = regex::Regex::new(r"eyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}").unwrap();
    out = re_jwt.replace_all(&out, "JWT_REDACTED").to_string();
    out
}

async fn openai_conflict_check(cfg: &Cfg, title: &str, sources: &[(String,String)]) -> Option<serde_json::Value> {
    let key = cfg.openai_key.as_ref()?;
    if key.is_empty() { return None; }
    // budget gate (very conservative)
    if !allow_budget(cfg, 1, 900) { return None; }

    let mut msg = String::new();
    msg.push_str("You are Press Oracle. Detect conflicting factual claims across sources. Return JSON with {conflict:boolean, severity:1-5, summary:string, conflicting_points:[{claim,source_a,source_b}]}.\n");
    msg.push_str(&format!("Title: {}\n", title));
    for (u, t) in sources.iter().take(4) {
        let clip = t.chars().take(2200).collect::<String>();
        msg.push_str(&format!("SOURCE {}:\n{}\n\n", u, clip));
    }
    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [
            {"role":"system","content":"Return ONLY valid JSON. No markdown."},
            {"role":"user","content": redact_secrets(&msg)}
        ],
        "temperature": 0.2,
        "max_tokens": 450
    });

    let http = reqwest::Client::builder().timeout(Duration::from_secs(12)).build().ok()?;
    let res = http.post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(key)
        .json(&body)
        .send().await.ok()?;
    let j = res.json::<serde_json::Value>().await.ok()?;
    let content = j.get("choices")?.get(0)?.get("message")?.get("content")?.as_str()?.to_string();
    serde_json::from_str::<serde_json::Value>(&content).ok()
}

fn extract_numeric_claims(text: &str) -> Vec<(String,f64)> {
    // very simple: capture numbers + short context window
    let re = regex::Regex::new(r"(?i)(\b\d{1,3}(?:,\d{3})*(?:\.\d+)?\b)").unwrap();
    let mut out = Vec::new();
    for m in re.find_iter(text).take(40) {
        let num = m.as_str().replace(",", "");
        if let Ok(v) = num.parse::<f64>() {
            let start = m.start().saturating_sub(24);
            let end = (m.end()+24).min(text.len());
            let ctx = text[start..end].to_string();
            out.push((ctx, v));
        }
    }
    out
}


#[derive(Deserialize)]
struct AnalyzeSourcesReq {
    article_id: String,
    outlet: String,
    title: Option<String>,
    urls: Vec<String>,
    sig: Option<String>,
}

async fn analyze_sources(
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    axum::extract::State(cfg): axum::extract::State<Cfg>,
    Json(req): Json<AnalyzeSourcesReq>
) -> Json<serde_json::Value> {
    if !allow_key(&format!("ip:{}", addr.ip()), 30, 60) { return Json(serde_json::json!({"ok": false, "error":"rate_limited"})); }

    let body = serde_json::json!({"article_id":req.article_id,"outlet":req.outlet,"title":req.title,"urls":req.urls}).to_string();
    if !require_sig(&cfg, req.sig.clone(), &body) {
        return Json(serde_json::json!({"ok": false, "error":"invalid signature"}));
    }

    let http = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().unwrap();
    let mut sources: Vec<(String,String)> = Vec::new();
    for u in req.urls.iter().take(5) {
        if let Ok(r) = http.get(u).send().await {
            if let Ok(t) = r.text().await {
                let txt = html_to_text(&t);
                sources.push((u.clone(), txt));
            }
        }
    }

    // Heuristic conflict check via numeric claim divergence
    let mut conflicts = Vec::new();
    if sources.len() >= 2 {
        let a = extract_numeric_claims(&sources[0].1);
        for (u, t) in sources.iter().skip(1) {
            let b = extract_numeric_claims(t);
            for (ctx_a, va) in a.iter().take(15) {
                for (ctx_b, vb) in b.iter().take(15) {
                    let rel = if *va == 0.0 { 0.0 } else { ((va-vb).abs()/va.abs()) };
                    if rel > 0.25 && ctx_a.split_whitespace().next() == ctx_b.split_whitespace().next() {
                        conflicts.push(serde_json::json!({"a": ctx_a, "b": ctx_b, "rel_diff": rel, "source_a": sources[0].0, "source_b": u}));
                    }
                }
            }
        }
    }

    let mut flags = Vec::new();
    if !conflicts.is_empty() {
        flags.push(serde_json::json!({
            "severity": 4,
            "kind": "conflict",
            "source": "ai",
            "details": {"conflicts": conflicts}
        }));
    }

    // Optional OpenAI high-precision conflict summary
    if let Some(j) = openai_conflict_check(&cfg, &req.title.clone().unwrap_or_else(|| "Press Article".into()), &sources).await {
        if j.get("conflict").and_then(|v| v.as_bool()).unwrap_or(false) {
            flags.push(serde_json::json!({
                "severity": j.get("severity").and_then(|v| v.as_i64()).unwrap_or(3),
                "kind": "conflict",
                "source": "openai",
                "details": j
            }));
        }
    }

    // Persist any conflict flags
    for f in flags.iter() {
        let sev = f.get("severity").and_then(|v| v.as_i64()).unwrap_or(3);
        let kind = f.get("kind").and_then(|v| v.as_str()).unwrap_or("conflict");
        let src = f.get("source").and_then(|v| v.as_str()).unwrap_or("ai");
        let details = f.get("details").cloned().unwrap_or(serde_json::json!({}));
        let _ = http.post(format!("{}/oracle/flag", cfg.indexer_api))
            .json(&serde_json::json!({
                "article_id": req.article_id,
                "severity": sev,
                "kind": kind,
                "source": src,
                "title": req.title,
                "url": req.urls.get(0),
                "details": details
            }))
            .send().await;
    }

    Json(serde_json::json!({"ok": true, "flags": flags}))
}
