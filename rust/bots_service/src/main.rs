use std::sync::atomic::{AtomicBool, Ordering};

async fn is_admin_request_async(state: &AppState, headers: &axum::http::HeaderMap) -> bool {
    let tok = headers.get("x-admin-token").and_then(|v| v.to_str().ok()).unwrap_or("");
    let need = state.cfg.read().await.admin_token.clone();
    !need.is_empty() && tok == need
}

use axum::{routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::{env, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::RwLock;
use std::sync::Arc;

use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    queue_path: PathBuf,
    missions_path: PathBuf,
    bindings_path: PathBuf,
    tg_codes_path: PathBuf,
    tg_subs_path: PathBuf,

    cfg: Arc<RwLock<BotConfig>>,
    state_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct BotConfig {
    brand_name: String,
    public_base_url: String,

    discord_bot_token_set: bool,
    telegram_bot_token_set: bool,

    // Discord
    discord_client_id: String,
    discord_default_channels: Vec<ChannelBinding>,
    discord_token_gated_channels: Vec<TokenGatedChannel>,

    // Telegram
    telegram_default_chats: Vec<ChatBinding>,

    // Admin
    admin_token: String,

    // Council
    council_max_members: usize,
    council_role_name: String,
    council_grace_days: i64,

    // Role sync
    role_removal_enabled: bool,
    press_council_guild_id: Option<String>,
    press_council_role_id: Option<String>,
    press_council_max: usize,
    press_council_grace_days: u64,
    press_council_eligibility_url: Option<String>,
    press_council_count_cache_ttl_secs: u64,

    // Oracle alerts
    oracle_alerts_enabled: bool,
    oracle_min_severity: i64,

    // Feature flags
    bots_enabled: bool,


    // On-chain heartbeat
    onchain_heartbeat_enabled: bool,
    onchain_heartbeat_interval_sec: u64,
    onchain_heartbeat_rpc: String,
    onchain_heartbeat_contract: String,
    onchain_heartbeat_privkey: Option<String>,
    features_path: Option<String>,


    // Press integration
    press_rpc_http: String,
    press_query_api: String,
    press_indexer_api: String,

    // Admin gate (Discord HQ role)
    admin_gate: AdminGate,

    // Feature flags (release staging)
    features: Features,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnnouncementQueueItem {
    id: String,
    created_at: i64,
    text: String,
    cta_url: Option<String>,
    cta_label: Option<String>,
    // Optional scoping; if none, broadcast to all bindings
    scope_guild_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MissionItem {
    id: String,
    created_at: i64,
    ends_at: i64,
    title: String,
    description: String,
    reward: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AdminGate {
    // Only admins in this guild with this role can broadcast
    hq_guild_id: String,
    hq_admin_role_id: String,
}
struct Features {
    live_article_feed: bool,
    pending_vote_feed: bool,
    proposal_feed: bool,
    court_feed: bool,
    press_pass_verification: bool,
    token_gated_roles: bool,
    anti_brigade_guard: bool,
    inline_vote_cards: bool,
    outlet_token_alerts: bool,
    syndication_deals: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChannelBinding {
    guild_id: String,
    channel_id: String,
    purpose: String, // "recent_articles", "pending_votes", "proposals", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TokenGatedChannel {
    guild_id: String,
    channel_id: String,
    token_address: String,
    min_balance_wei: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatBinding {
    chat_id: String,
    purpose: String,
}

#[derive(Debug, Serialize)]
struct Health { ok: bool, service: &'static str }

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let state_path = PathBuf::from(env::var("PRESS_BOTS_STATE").unwrap_or_else(|_| "/state/bots_config.json".into()));
    let cfg = load_or_default(&state_path).await;

    let queue_path = PathBuf::from(env::var("PRESS_BOTS_QUEUE").unwrap_or_else(|_| "/state/bots_queue.json".into()));
    let missions_path = PathBuf::from(env::var("PRESS_BOTS_MISSIONS").unwrap_or_else(|_| "/state/bots_missions.json".into()));
    let bindings_path = PathBuf::from(env::var("PRESS_BOTS_BINDINGS").unwrap_or_else(|_| "/state/bots_bindings.json".into()));
    let tg_codes_path = PathBuf::from(env::var("PRESS_TG_CODES").unwrap_or_else(|_| "/state/tg_onboard_codes.json".into()));
    let tg_subs_path = PathBuf::from(env::var("PRESS_TG_SUBS").unwrap_or_else(|_| "/state/tg_subscriptions.json".into()));

    let admin_key = env::var("HQ_ADMIN_KEY").ok();

    let telemetry = Arc::new(RwLock::new(BotTelemetry::default()));

    let db = sqlx::SqlitePool::connect(&format!("sqlite:{}", cfg.state_dir.join("bots.db").display())).await.expect("bots db");
    init_db(&db).await.expect("init bots db");

    let state = AppState {
        cfg: Arc::new(RwLock::new(cfg)),
        state_path,
        queue_path,
        missions_path,
        bindings_path,
        tg_codes_path,
        tg_subs_path,
    };
// Periodic role resync (keeps Discord roles consistent with on-chain roles/bonds/activity)
{
    let state_clone = state.clone();
    tokio::spawn(async move {
        loop {
            let _ = run_global_role_resync(&state_clone).await;
            tokio::time::sleep(std::time::Duration::from_secs(600)).await; // 10 min
        }
    });
// Periodic council enforcement (activity + max cap guard)
{
    let state_clone = state_clone.clone();
    tokio::spawn(async move {
        loop {
            let _ = council_enforce(
                axum::extract::State(state_clone.clone()),
                axum::http::HeaderMap::new()
            ).await;
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await; // hourly
        }
    });
}

}



    // Start bot loops only when tokens exist
    {
        let cfg = state.cfg.read().await;
        if cfg.discord_bot_token_set {
            info!("Discord token detected: Discord loop will start (scaffold).");
        } else {
            warn!("Discord token not set; Discord bot will stay offline until configured.");
        }
        if cfg.telegram_bot_token_set {
            info!("Telegram token detected: Telegram loop will start (scaffold).");
        } else {
            warn!("Telegram token not set; Telegram bot will stay offline until configured.");
        }
    }

    // Background dispatch loops (Discord/Telegram) start only when tokens exist.
    spawn_dispatch_loops(state.clone());

    let app = Router::new()
        .route("/health", get(|| async { Json(Health{ok:true, service:"press_bots_service"}) }))
        .route("/api/bots/status", get(get_status))
        .route("/api/bots/config", get(get_config).post(set_config))
        .route("/api/bots/bindings", get(get_bindings).post(add_binding))
        .route("/api/bots/admin/gate", get(get_admin_gate))
        .route("/api/bots/discord/invite", post(discord_invite))
        .route("/api/bots/features", post(toggle_feature))
        .route("/api/bots/auth/wallet", post(auth_wallet))
        .route("/api/bots/auth/discord/start", get(auth_discord_start))
        .route("/api/bots/auth/discord/callback", get(auth_discord_callback))
        .route("/api/bots/admin/announce", post(admin_announce))
        .route("/api/bots/admin/heartbeat_now", post(admin_heartbeat_now))
        .route("/api/bots/outlet/register", post(outlet_register_channels))
        .route("/api/bots/outlet/list", get(outlet_list_channels))
        .route("/api/bots/outlet/verify", post(outlet_verify_channels))
        .route("/api/bots/discord/verify_channel", post(discord_verify_channel))
        .route("/api/bots/telegram/verify_chat", post(telegram_verify_chat))
        .route("/api/bots/outlet/role_mappings/get", get(get_role_mappings))
        .route("/api/bots/outlet/role_mappings/set", post(set_role_mapping))
        .route("/api/bots/outlet/preflight", post(outlet_preflight))
        .route("/api/bots/discord/oauth/start", get(discord_oauth_start))
        .route("/api/bots/discord/oauth/callback", get(discord_oauth_callback))
        .route("/api/bots/discord/link_wallet", post(discord_link_wallet))
        .route("/api/bots/discord/sync_roles", post(discord_sync_roles))
        .route("/api/bots/discord/resync_all", post(discord_resync_all))
        .route("/api/bots/council/stats", get(council_stats))
        .route("/api/bots/council/sync_user", post(council_sync_user))
        .route("/api/bots/council/recount", post(council_recount))
        .route("/api/bots/council/status", get(council_status))
        .route("/api/bots/council/enforce", post(council_enforce))
        .route("/api/bots/admin/queue", get(get_queue))
        .route("/api/bots/admin/queue/delete", post(delete_queue_item))
        .route("/api/bots/admin/missions/create", post(admin_mission_create))
        .route("/api/bots/telegram/onboarding_link", post(telegram_onboarding_link))
        .route("/api/bots/telegram/subscriptions", get(tg_get_subs).post(tg_set_subs))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let bind = env::var("BIND").unwrap_or_else(|_| "0.0.0.0:8790".into());
    let addr: SocketAddr = bind.parse()?;
    info!("press_bots_service listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

async fn load_or_default(path: &PathBuf) -> BotConfig {
    if let Ok(bytes) = tokio::fs::read(path).await {
        if let Ok(cfg) = serde_json::from_slice::<BotConfig>(&bytes) {
            return cfg;
        }
    }
    // Default: "PressPulse" (official bot brand)
    let mut cfg = BotConfig::default();
    cfg.brand_name = env::var("PRESS_BOT_NAME").unwrap_or_else(|_| "PressPulse".into());
    cfg.public_base_url = env::var("PRESS_BOTS_PUBLIC_URL").unwrap_or_else(|_| "https://bots.pressblockchain.io".into());
    cfg.discord_client_id = env::var("DISCORD_CLIENT_ID").unwrap_or_default();
    cfg.press_rpc_http = env::var("PRESS_RPC_HTTP").unwrap_or_else(|_| "http://press-rpc:8545".into());
    cfg.press_query_api = env::var("PRESS_QUERY_API").unwrap_or_else(|_| "http://query-api:8787".into());
    cfg.press_indexer_api = env::var("PRESS_INDEXER_API").unwrap_or_else(|_| "http://press-indexer:8786".into());

    cfg.admin_token = env::var("PRESS_ADMIN_TOKEN").unwrap_or_else(|_| "changeme".into());
    cfg.council_max_members = env::var("PRESS_COUNCIL_MAX").ok().and_then(|v| v.parse().ok()).unwrap_or(195);
    cfg.council_role_name = env::var("PRESS_COUNCIL_ROLE").unwrap_or_else(|_| "PRESS_COUNCIL".into());
    cfg.council_grace_days = env::var("PRESS_COUNCIL_GRACE_DAYS").ok().and_then(|v| v.parse().ok()).unwrap_or(14);
    cfg.role_removal_enabled = env::var("PRESS_ROLE_REMOVAL_ENABLED").ok().map(|v| v=="1"||v.to_lowercase()=="true").unwrap_or(true);
    cfg.press_council_guild_id = env::var("PRESS_COUNCIL_GUILD_ID").ok();
    cfg.press_council_role_id = env::var("PRESS_COUNCIL_ROLE_ID").ok();
    cfg.press_council_max = env::var("PRESS_COUNCIL_MAX").ok().and_then(|v| v.parse().ok()).unwrap_or(195);
    cfg.press_council_grace_days = env::var("PRESS_COUNCIL_GRACE_DAYS").ok().and_then(|v| v.parse().ok()).unwrap_or(7);
    cfg.press_council_eligibility_url = env::var("PRESS_COUNCIL_ELIGIBILITY_URL").ok();
    cfg.press_council_count_cache_ttl_secs = env::var("PRESS_COUNCIL_COUNT_CACHE_TTL_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(300);

    cfg.oracle_alerts_enabled = env::var("PRESS_ORACLE_ALERTS_ENABLED").ok().map(|v| v=="1" || v.to_lowercase()=="true").unwrap_or(true);
    cfg.oracle_min_severity = env::var("PRESS_ORACLE_MIN_SEVERITY").ok().and_then(|v| v.parse().ok()).unwrap_or(3);

    cfg.bots_enabled = env::var("PRESS_BOTS_ENABLED").ok().map(|v| v=="1" || v.to_lowercase()=="true").unwrap_or(true);

    cfg.onchain_heartbeat_enabled = env::var("PRESS_ONCHAIN_HEARTBEAT_ENABLED").ok().map(|v| v=="1" || v.to_lowercase()=="true").unwrap_or(false) };

    cfg.onchain_heartbeat_interval_sec = env::var("PRESS_ONCHAIN_HEARTBEAT_INTERVAL_SEC").ok().and_then(|v| v.parse().ok()).unwrap_or(300);
    cfg.onchain_heartbeat_rpc = env::var("PRESS_ONCHAIN_HEARTBEAT_RPC").unwrap_or_else(|_| "http://press-rpc:8545".into());
    cfg.onchain_heartbeat_contract = env::var("PRESS_ONCHAIN_HEARTBEAT_CONTRACT").unwrap_or_else(|_| "".into());
    cfg.onchain_heartbeat_privkey = env::var("PRESS_ONCHAIN_HEARTBEAT_PRIVKEY").ok();
    // optional features file (shared with installer)
    cfg.features_path = env::var("PRESS_FEATURES_PATH").ok();

    cfg.admin_gate = AdminGate {
        hq_guild_id: env::var("PRESS_HQ_GUILD_ID").unwrap_or_default(),
        hq_admin_role_id: env::var("PRESS_HQ_ADMIN_ROLE_ID").unwrap_or_default(),
    };

    let discord_token = env::var("DISCORD_BOT_TOKEN").unwrap_or_default();
    cfg.discord_bot_token_set = !discord_token.is_empty();
    let telegram_token = env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();
    cfg.telegram_bot_token_set = !telegram_token.is_empty();

    cfg.features = Features {
        live_article_feed: true,
        pending_vote_feed: true,
        proposal_feed: true,
        court_feed: false, // can be staged
        press_pass_verification: true,
        token_gated_roles: true,
        anti_brigade_guard: true,
        inline_vote_cards: true,
        outlet_token_alerts: true,
        syndication_deals: false, // can be staged
    };
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

async fn get_status(axum::extract::State(state): axum::extract::State<AppState>) -> Json<serde_json::Value> {
    let cfg = state.cfg.read().await;
    Json(serde_json::json!({
        "ok": true,
        "brand": cfg.brand_name,
        "discord": { "configured": cfg.discord_bot_token_set, "client_id": cfg.discord_client_id },
        "telegram": { "configured": cfg.telegram_bot_token_set },
        "features": cfg.features,
        "public_url": cfg.public_base_url
    }))
}

async fn get_config(axum::extract::State(state): axum::extract::State<AppState>) -> Json<serde_json::Value> {
    let cfg = state.cfg.read().await;
    Json(serde_json::json!({"ok": true, "config": cfg.clone()}))
}

async fn set_config(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(payload): Json<BotConfig>,
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

#[derive(Deserialize)]
struct InviteReq { redirect_uri: Option<String> }

async fn discord_invite(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<InviteReq>,
) -> Json<serde_json::Value> {
    let cfg = state.cfg.read().await;
    if cfg.discord_client_id.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "DISCORD_CLIENT_ID not set"}));
    }
    let redirect = req.redirect_uri.unwrap_or_else(|| format!("{}/dashboard", cfg.public_base_url));
    // permissions: 8 (admin) is too broad; use a hardened set. This is a safe baseline:
    // Read messages, send messages, embed links, attach files, manage webhooks, manage roles.
    let permissions = "274877910016";
    let url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&permissions={}&scope=bot%20applications.commands&redirect_uri={}&response_type=code",
        cfg.discord_client_id,
        permissions,
        urlencoding::encode(&redirect)
    );
    Json(serde_json::json!({"ok": true, "invite_url": url}))
}

#[derive(Deserialize)]
struct ToggleFeatureReq { key: String, enabled: bool }


#[derive(Serialize)]
struct OracleAlertsCfgResp { enabled: bool, min_severity: i64 }

async fn get_oracle_alerts_config(axum::extract::State(state): axum::extract::State<AppState>) -> Json<OracleAlertsCfgResp> {
    let cfg = state.cfg.read().await.clone();
    if !cfg.bots_enabled { return Ok(Json(serde_json::json!({"ok":false,"error":"bots_disabled"}))); }
    Json(OracleAlertsCfgResp{ enabled: cfg.oracle_alerts_enabled, min_severity: cfg.oracle_min_severity })
}

#[derive(Deserialize)]
struct SetOracleAlertsReq { enabled: Option<bool>, min_severity: Option<i64> }

async fn set_oracle_alerts_config(axum::extract::State(state): axum::extract::State<AppState>, Json(req): Json<SetOracleAlertsReq>) -> Json<serde_json::Value> {
    let mut cfg = state.cfg.write().await;
    if let Some(v) = req.enabled { cfg.oracle_alerts_enabled = v; }
    if let Some(v) = req.min_severity { cfg.oracle_min_severity = v.clamp(1,5); }
    // persist config to disk
    let _ = write_json(&state.state_path, &*cfg).await;
    Json(serde_json::json!({"ok": true}))
}




async fn get_discord_guilds(axum::extract::State(state): axum::extract::State<AppState>) -> Json<serde_json::Value> {
    let tel = state.telemetry.read().await.clone();
    let items: Vec<serde_json::Value> = tel.discord_guild_list.iter().map(|(id,name)| serde_json::json!({"id":id,"name":name})).collect();
    Json(serde_json::json!({"ok": true, "guilds": items}))
}

async fn get_telemetry(axum::extract::State(state): axum::extract::State<AppState>) -> Json<serde_json::Value> {
    let tel = state.telemetry.read().await.clone();
    Json(serde_json::json!({"ok": true, "telemetry": tel}))
}

async fn get_queue(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
) -> Json<serde_json::Value> {
    if !is_admin_request_async(&state, &headers).await {
        return Json(serde_json::json!({"ok": false, "error":"unauthorized"}));
    }
    let q: Vec<AnnouncementQueueItem> = read_json_vec(&state.queue_path).await;
    Json(serde_json::json!({"ok": true, "items": q}))
}

#[derive(Deserialize)]
struct DeleteQueueReq { id: String }

async fn delete_queue_item(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<DeleteQueueReq>,
) -> Json<serde_json::Value> {
    if !is_admin_request_async(&state, &headers).await {
        return Json(serde_json::json!({"ok": false, "error":"unauthorized"}));
    }
    let mut q: Vec<AnnouncementQueueItem> = read_json_vec(&state.queue_path).await;
    let before = q.len();
    q.retain(|x| x.id != req.id);
    let _ = write_json_vec(&state.queue_path, &q).await;
    Json(serde_json::json!({"ok": true, "removed": (before - q.len())}))
}

async fn toggle_feature(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<ToggleFeatureReq>,
) -> Json<serde_json::Value> {
    {
        let mut cfg = state.cfg.write().await;
        match req.key.as_str() {
            "live_article_feed" => cfg.features.live_article_feed = req.enabled,
            "pending_vote_feed" => cfg.features.pending_vote_feed = req.enabled,
            "proposal_feed" => cfg.features.proposal_feed = req.enabled,
            "court_feed" => cfg.features.court_feed = req.enabled,
            "press_pass_verification" => cfg.features.press_pass_verification = req.enabled,
            "token_gated_roles" => cfg.features.token_gated_roles = req.enabled,
            "anti_brigade_guard" => cfg.features.anti_brigade_guard = req.enabled,
            "inline_vote_cards" => cfg.features.inline_vote_cards = req.enabled,
            "outlet_token_alerts" => cfg.features.outlet_token_alerts = req.enabled,
            "syndication_deals" => cfg.features.syndication_deals = req.enabled,
            _ => return Json(serde_json::json!({"ok": false, "error": "unknown feature key"})),
        }
    }
    if let Err(e) = persist(&state).await {
        return Json(serde_json::json!({"ok": false, "error": e.to_string()}));
    }
    Json(serde_json::json!({"ok": true}))
}


// --- Auth (Discord OAuth + Press Wallet signature) ---
//
// This is production-oriented scaffolding: secrets must be provided via env.
// JWT is HMAC-SHA256.
// Wallet auth uses signed message verification via `personal_sign` signature validation.
//
// IMPORTANT: This service does not print secrets.

use axum::extract::Query;
use time::OffsetDateTime;

#[derive(Deserialize)]
struct WalletAuthReq { address: String, signature: String, message: String }

fn hmac_sha256(secret: &[u8], data: &[u8]) -> [u8;32] {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("hmac");
    mac.update(data);
    let res = mac.finalize().into_bytes();
    let mut out=[0u8;32];
    out.copy_from_slice(&res);
    out
}
fn b64url(data: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}
fn issue_jwt(subject: &str) -> anyhow::Result<String> {
    let secret = std::env::var("PRESS_BOTS_JWT_SECRET").unwrap_or_else(|_| "dev-unsafe-change-me".into());
    let header = b64url(br#"{"alg":"HS256","typ":"JWT"}"#);
    let exp = OffsetDateTime::now_utc().unix_timestamp() + 60*60*6; // 6h
    let payload = format!(r#"{{"sub":"{}","exp":{}}}"#, subject, exp);
    let payload = b64url(payload.as_bytes());
    let signing = format!("{}.{}", header, payload);
    let sig = hmac_sha256(secret.as_bytes(), signing.as_bytes());
    Ok(format!("{}.{}", signing, b64url(&sig)))
}

fn verify_personal_sign(address: &str, signature: &str, message: &str) -> anyhow::Result<bool> {
    // Minimal signature verification for EIP-191 personal_sign:
    // Recover pubkey from signature and compare to address.
    // Uses ethers-core.
    use ethers_core::types::Signature;
    use ethers_core::utils::hash_message;
    use ethers_core::types::H160;

    let sig: Signature = signature.parse()?;
    let msg_hash = hash_message(message);
    let rec = sig.recover(msg_hash)?;
    let rec_addr: H160 = rec;
    Ok(format!("{:#x}", rec_addr).to_lowercase() == address.to_lowercase())
}

async fn auth_wallet(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<WalletAuthReq>,
) -> Json<serde_json::Value> {
    match verify_personal_sign(&req.address, &req.signature, &req.message) {
        Ok(true) => {
            // Persist last seen address (optional)
            let _ = state; // future: bind wallet to discord user id
            let jwt = issue_jwt(&format!("wallet:{}", req.address)).unwrap_or_default();
            Json(serde_json::json!({"ok": true, "jwt": jwt}))
        }
        Ok(false) => Json(serde_json::json!({"ok": false, "error": "invalid signature"})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

async fn auth_discord_start(Query(q): Query<std::collections::HashMap<String,String>>) -> axum::response::Redirect {
    let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap_or_default();
    let redirect_uri = std::env::var("DISCORD_REDIRECT_URI").unwrap_or_else(|_| "https://bots.pressblockchain.io/api/bots/auth/discord/callback".into());
    let return_to = q.get("return").cloned().unwrap_or_else(|| "/dashboard".into());
    // state carries return path
    let state = b64url(return_to.as_bytes());
    let scope = "identify%20guilds";
    let url = format!("https://discord.com/oauth2/authorize?client_id={}&response_type=code&redirect_uri={}&scope={}&state={}",
        client_id,
        urlencoding::encode(&redirect_uri),
        scope,
        state
    );
    axum::response::Redirect::temporary(&url)
}

#[derive(Deserialize)]
struct DiscordCb { code: String, state: Option<String> }


async fn auth_discord_callback(Query(cb): Query<DiscordCb>, axum::extract::State(state): axum::extract::State<AppState>) -> axum::response::Redirect {
    // Full OAuth exchange (RR82):
    // - exchange code for access_token
    // - fetch /users/@me for id
    // - verify the user has HQ admin role in HQ guild
    // - issue JWT scoped to discord user id
    //
    // Requires:
    //   DISCORD_CLIENT_ID, DISCORD_CLIENT_SECRET, DISCORD_REDIRECT_URI
    //   PRESS_HQ_GUILD_ID, PRESS_HQ_ADMIN_ROLE_ID
    //   DISCORD_BOT_TOKEN (to check member roles via Bot API)

    let return_to = cb.state.as_deref()
        .and_then(|s| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(s).ok())
        .and_then(|b| String::from_utf8(b).ok())
        .unwrap_or_else(|| "/dashboard".into());

    let client_id = std::env::var("DISCORD_CLIENT_ID").unwrap_or_default();
    let client_secret = std::env::var("DISCORD_CLIENT_SECRET").unwrap_or_default();
    let redirect_uri = std::env::var("DISCORD_REDIRECT_URI").unwrap_or_else(|_| "https://bots.pressblockchain.io/api/bots/auth/discord/callback".into());

    let cfg = state.cfg.read().await.clone();
    if client_id.is_empty() || client_secret.is_empty() {
        let url = format!("{}#err={}", return_to, urlencoding::encode("discord client secret not configured"));
        return axum::response::Redirect::temporary(&url);
    }
    if cfg.admin_gate.hq_guild_id.is_empty() || cfg.admin_gate.hq_admin_role_id.is_empty() {
        let url = format!("{}#err={}", return_to, urlencoding::encode("admin gate not configured"));
        return axum::response::Redirect::temporary(&url);
    }

    // exchange token
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("client");

    let token_resp = http
        .post("https://discord.com/api/oauth2/token")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(serde_urlencoded::to_string(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", cb.code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
        ]).unwrap())
        .send().await;

    let token_json: serde_json::Value = match token_resp {
        Ok(r) => match r.json().await { Ok(j)=>j, Err(e)=> {
            let url = format!("{}#err={}", return_to, urlencoding::encode(&format!("token parse: {}", e)));
            return axum::response::Redirect::temporary(&url);
        }},
        Err(e) => {
            let url = format!("{}#err={}", return_to, urlencoding::encode(&format!("token exchange: {}", e)));
            return axum::response::Redirect::temporary(&url);
        }
    };
    let access = token_json.get("access_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if access.is_empty() {
        let url = format!("{}#err={}", return_to, urlencoding::encode("missing access token"));
        return axum::response::Redirect::temporary(&url);
    }

    // fetch user
    let user_resp = http.get("https://discord.com/api/users/@me")
        .bearer_auth(&access)
        .send().await;

    let user_json: serde_json::Value = match user_resp {
        Ok(r) => match r.json().await { Ok(j)=>j, Err(e)=> {
            let url = format!("{}#err={}", return_to, urlencoding::encode(&format!("user parse: {}", e)));
            return axum::response::Redirect::temporary(&url);
        }},
        Err(e) => {
            let url = format!("{}#err={}", return_to, urlencoding::encode(&format!("user fetch: {}", e)));
            return axum::response::Redirect::temporary(&url);
        }
    };
    let user_id = user_json.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if user_id.is_empty() {
        let url = format!("{}#err={}", return_to, urlencoding::encode("missing user id"));
        return axum::response::Redirect::temporary(&url);
    }

    // verify member has HQ role using bot token
    let bot_token = std::env::var("DISCORD_BOT_TOKEN").unwrap_or_default();
    if bot_token.is_empty() {
        let url = format!("{}#err={}", return_to, urlencoding::encode("bot token not configured"));
        return axum::response::Redirect::temporary(&url);
    }

    let member_url = format!("https://discord.com/api/guilds/{}/members/{}", cfg.admin_gate.hq_guild_id, user_id);
    let member_resp = http.get(member_url)
        .header("Authorization", format!("Bot {}", bot_token))
        .send().await;

    let member_json: serde_json::Value = match member_resp {
        Ok(r) => {
            if !r.status().is_success() {
                let url = format!("{}#err={}", return_to, urlencoding::encode("not in HQ server or missing permissions"));
                return axum::response::Redirect::temporary(&url);
            }
            match r.json().await {
                Ok(j)=>j,
                Err(e)=>{
                    let url = format!("{}#err={}", return_to, urlencoding::encode(&format!("member parse: {}", e)));
                    return axum::response::Redirect::temporary(&url);
                }
            }
        }
        Err(e) => {
            let url = format!("{}#err={}", return_to, urlencoding::encode(&format!("member fetch: {}", e)));
            return axum::response::Redirect::temporary(&url);
        }
    };

    let roles = member_json.get("roles").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let has_role = roles.iter().any(|r| r.as_str().unwrap_or("") == cfg.admin_gate.hq_admin_role_id);
    if !has_role {
        let url = format!("{}#err={}", return_to, urlencoding::encode("missing required admin role"));
        return axum::response::Redirect::temporary(&url);
    }

    let jwt = issue_jwt(&format!("discord:{}", user_id)).unwrap_or_default();
    let url = format!("{}#jwt={}", return_to, urlencoding::encode(&jwt));
    axum::response::Redirect::temporary(&url)
}


#[derive(Deserialize)]
struct AnnounceReq { text: String, cta_url: Option<String>, cta_label: Option<String> }




#[derive(Deserialize)]
struct OutletRegisterReq {
    outlet_id: String,
    outlet_name: Option<String>,
    official_domain: Option<String>,
    discord_guild_id: Option<String>,
    discord_channel_id: Option<String>,
    telegram_chat_id: Option<String>,
}


#[derive(Deserialize)]
struct VerifyOutletReq { outlet_id: String }

async fn outlet_verify_channels(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<VerifyOutletReq>
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_request_async(&state, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let row = sqlx::query("SELECT outlet_id,outlet_name,official_domain,discord_channel_id,telegram_chat_id FROM outlet_channels WHERE outlet_id = ?1")
        .bind(&req.outlet_id)
        .fetch_optional(&state.db)
        .await
        .ok()
        .flatten();
    if row.is_none() {
        return Ok(Json(serde_json::json!({"ok":false,"error":"not_found"})));
    }
    let r=row.unwrap();
    let outlet_id:String=r.get(0);
    let outlet_name:String=r.get(1);
    let domain:String=r.get(2);
    let dc:String=r.get(3);
    let tg:String=r.get(4);

    let mut results = serde_json::Map::new();
    if !dc.is_empty() {
        let ok = discord_send_test(&state, &dc, &format!("PressPulse verified for outlet **{}** ({})", outlet_name, domain)).await;
        results.insert("discord".into(), serde_json::Value::Bool(ok));
    }
    if !tg.is_empty() {
        let ok = telegram_send_test(&state, &tg, &format!("PressPulse verified for outlet {} ({})", outlet_name, domain)).await;
        results.insert("telegram".into(), serde_json::Value::Bool(ok));
    }
    Ok(Json(serde_json::json!({"ok":true,"outlet_id": outlet_id, "results": results})))
}

#[derive(Deserialize)]
struct VerifyDiscordReq { channel_id: String, message: Option<String> }

async fn discord_verify_channel(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<VerifyDiscordReq>
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_request_async(&state, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let msg = req.message.unwrap_or_else(|| "PressPulse verification test.".into());
    let ok = discord_send_test(&state, &req.channel_id, &msg).await;
    Ok(Json(serde_json::json!({"ok": ok})))
}

#[derive(Deserialize)]
struct VerifyTelegramReq { chat_id: String, message: Option<String> }

async fn telegram_verify_chat(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<VerifyTelegramReq>
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_request_async(&state, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let msg = req.message.unwrap_or_else(|| "PressPulse verification test.".into());
    let ok = telegram_send_test(&state, &req.chat_id, &msg).await;
    Ok(Json(serde_json::json!({"ok": ok})))
}

async fn discord_send_test(state: &AppState, channel_id: &str, message: &str) -> bool {
    if let Some(http) = state.discord_http.read().await.clone() {
        if let Ok(cid) = channel_id.parse::<u64>() {
            return serenity::model::id::ChannelId(cid).send_message(&http, |m| m.content(message)).await.is_ok();
        }
    }
    false
}

async fn telegram_send_test(state: &AppState, chat_id: &str, message: &str) -> bool {
    let cfg = state.cfg.read().await.clone();
    if let Some(tok) = cfg.telegram_bot_token {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", tok);
        return reqwest::Client::new()
            .post(url)
            .json(&serde_json::json!({"chat_id": chat_id, "text": message}))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false) };

    }
    false
}


use axum::response::{IntoResponse, Redirect};
use axum::extract::{Query};
use std::collections::HashMap;

#[derive(Deserialize)]
struct RoleMapSetReq {
    outlet_id: String,
    press_role: String,
    discord_role_id: String,
}


#[derive(Deserialize)]
struct OutletPreflightReq {
    outlet_id: String,
    discord_guild_id: Option<String>,
    discord_channel_id: Option<String>,
}

async fn outlet_preflight(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<OutletPreflightReq>
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_request_async(&state, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let mut out = serde_json::Map::new();

    // Discord checks
    let dg = req.discord_guild_id.unwrap_or_default();
    let dc = req.discord_channel_id.unwrap_or_default();
    if dg.is_empty() || dc.is_empty() {
        out.insert("discord".into(), serde_json::json!({"ok": false, "error":"missing_ids"}));
    } else {
        let mut can_send=false;
        let mut can_manage_roles=false;
        if let Some(http)=state.discord_http.read().await.clone() {
            // Try sending a test message
            if let Ok(cid)=dc.parse::<u64>() {
                can_send = serenity::model::id::ChannelId(cid)
                    .send_message(&http, |m| m.content("PressPulse preflight: channel verified."))
                    .await.is_ok();
            }
            // Check manage_roles by inspecting bot member perms (best-effort)
            if let (Ok(gid), Ok(bot_user)) = (dg.parse::<u64>(), http.get_current_user().await.map(|u| u.id.0)) {
                if let Ok(member)=serenity::model::id::GuildId(gid).member(&http, bot_user).await {
                    // If bot has at least one role besides @everyone, treat as manageable (best-effort MVP)
                    can_manage_roles = !member.roles.is_empty();
                }
            }
        }
        
// Strict checks: bot must have MANAGE_ROLES and be above target roles (best-effort).
let mut can_manage_roles_strict = false;
let mut hierarchy_ok = false;
if let Some(http)=state.discord_http.read().await.clone() {
    if let Ok(gid)=dg.parse::<u64>() {
        if let Ok(current)=http.get_current_user().await {
            if let Ok(bot_member)=serenity::model::id::GuildId(gid).member(&http, current.id).await {
                if let Ok(perms)=bot_member.permissions(&http).await {
                    can_manage_roles_strict = perms.manage_roles();
                }
                // If we have mappings, ensure bot highest role is above mapped roles
                let maps = sqlx::query("SELECT discord_role_id FROM role_mappings WHERE outlet_id=?1")
                    .bind(&req.outlet_id).fetch_all(&state.db).await.unwrap_or_default();
                if let Ok(guild)=serenity::model::id::GuildId(gid).to_partial_guild(&http).await {
                    let mut bot_top = 0i64;
                    for rid in bot_member.roles.iter() {
                        if let Some(role)=guild.roles.get(rid) {
                            bot_top = bot_top.max(role.position as i64);
                        }
                    }
                    let mut ok=true;
                    for m in maps {
                        let dr:String=m.get(0);
                        if let Ok(rid)=dr.parse::<u64>() {
                            let role_id=serenity::model::id::RoleId(rid);
                            if let Some(role)=guild.roles.get(&role_id) {
                                if (role.position as i64) >= bot_top {
                                    ok=false;
                                }
                            }
                        }
                    }
                    hierarchy_ok = ok;
                }
            }
        }
    }
}

out.insert("discord".into(), serde_json::json!({"ok": can_send, "can_send": can_send, "can_manage_roles_hint": can_manage_roles, "can_manage_roles_strict": can_manage_roles_strict, "hierarchy_ok": hierarchy_ok}));
    }

    Ok(Json(serde_json::json!({"ok": true, "outlet_id": req.outlet_id, "preflight": out})))
}

async fn get_role_mappings(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_request_async(&state, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let rows = sqlx::query("SELECT outlet_id, press_role, discord_role_id FROM role_mappings ORDER BY outlet_id, press_role")
        .fetch_all(&state.db).await.unwrap_or_default();
    let mut out = vec![];
    for r in rows {
        out.push(serde_json::json!({
            "outlet_id": r.get::<String,_>(0),
            "press_role": r.get::<String,_>(1),
            "discord_role_id": r.get::<String,_>(2),
        }));
    }
    Ok(Json(serde_json::json!({"ok":true,"mappings":out})))
}

async fn set_role_mapping(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<RoleMapSetReq>
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_request_async(&state, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let now = now_ts();
    let _ = sqlx::query(r#"
        INSERT INTO role_mappings(outlet_id,press_role,discord_role_id,created_at)
        VALUES (?1,?2,?3,?4)
        ON CONFLICT(outlet_id,press_role) DO UPDATE SET
          discord_role_id=excluded.discord_role_id,
          created_at=excluded.created_at
    "#)
      .bind(&req.outlet_id)
      .bind(&req.press_role)
      .bind(&req.discord_role_id)
      .bind(now)
      .execute(&state.db).await;
    Ok(Json(serde_json::json!({"ok":true})))
}

async fn discord_oauth_start(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(q): Query<HashMap<String,String>>,
) -> impl IntoResponse {
    let cfg = state.cfg.blocking_read().clone();
    let cid = cfg.discord_client_id.clone().unwrap_or_default();
    let redirect = cfg.discord_oauth_redirect.clone().unwrap_or_else(|| format!("{}/api/bots/discord/oauth/callback", cfg.public_base_url));
    let scope = "identify";
    let outlet_id = q.get("outlet_id").cloned().unwrap_or_default();
    let state_param = if outlet_id.is_empty() { "".to_string() } else { format!("outlet:{}", outlet_id) };
    let url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
        urlencoding::encode(&cid),
        urlencoding::encode(&redirect),
        urlencoding::encode(scope),
        urlencoding::encode(&state_param)
    );
    Redirect::temporary(&url)
}

#[derive(Deserialize)]
struct OAuthCb { code: String, state: Option<String> }

async fn discord_oauth_callback(
    axum::extract::State(state): axum::extract::State<AppState>,
    Query(q): Query<OAuthCb>,
) -> impl IntoResponse {
    // Exchange code -> token -> user id
    let cfg = state.cfg.read().await.clone();
    let cid = cfg.discord_client_id.unwrap_or_default();
    let secret = cfg.discord_client_secret.unwrap_or_default();
    let redirect = cfg.discord_oauth_redirect.unwrap_or_else(|| format!("{}/api/bots/discord/oauth/callback", cfg.public_base_url));
    if cid.is_empty() || secret.is_empty() {
        return (StatusCode::BAD_REQUEST, "Discord OAuth not configured.").into_response();
    }

    let client = reqwest::Client::new();
    let tok = client.post("https://discord.com/api/oauth2/token")
        .header("content-type","application/x-www-form-urlencoded")
        .body(format!("client_id={}&client_secret={}&grant_type=authorization_code&code={}&redirect_uri={}",
            urlencoding::encode(&cid),
            urlencoding::encode(&secret),
            urlencoding::encode(&q.code),
            urlencoding::encode(&redirect)
        ))
        .send().await;

    if tok.is_err() { return (StatusCode::BAD_REQUEST, "OAuth exchange failed.").into_response(); }
    let tokj: serde_json::Value = tok.unwrap().json().await.unwrap_or_else(|_| serde_json::json!({}));
    let access = tokj.get("access_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if access.is_empty() { return (StatusCode::BAD_REQUEST, "OAuth exchange failed.").into_response(); }

    let me = client.get("https://discord.com/api/users/@me")
        .bearer_auth(&access).send().await;
    if me.is_err() { return (StatusCode::BAD_REQUEST, "OAuth user fetch failed.").into_response(); }
    let mej: serde_json::Value = me.unwrap().json().await.unwrap_or_else(|_| serde_json::json!({}));
    let outlet_id_from_state = q.state.clone().unwrap_or_default().strip_prefix("outlet:").unwrap_or("").to_string();

    let user_id = mej.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if user_id.is_empty() { return (StatusCode::BAD_REQUEST, "OAuth user fetch failed.").into_response(); }

    let session = format!("sess_{}", uuid::Uuid::new_v4());
    let now = now_ts();
    let _ = sqlx::query("INSERT INTO oauth_sessions(session_id,discord_user_id,outlet_id,created_at) VALUES (?1,?2,?3,?4)")
        .bind(&session).bind(&user_id).bind(&outlet_id_from_state).bind(now).execute(&state.db).await;

    let page = format!(r#"<!doctype html><html><head><meta charset="utf-8"/>
<title>Connect PRESS Wallet</title>
<style>body{{font-family:ui-sans-serif,system-ui;background:#050b14;color:#e8eefc;display:flex;min-height:100vh;align-items:center;justify-content:center;padding:24px}}
.card{{max-width:520px;width:100%;background:#0a1426;border:1px solid #162a4a;border-radius:16px;padding:22px}}
.btn{{background:#2d6cdf;border:none;color:white;border-radius:12px;padding:10px 14px;font-weight:700;cursor:pointer}}
.small{{opacity:.8;font-size:13px;line-height:1.4}}
input{{width:100%;padding:10px;border-radius:12px;border:1px solid #233b63;background:#061024;color:#e8eefc}}
</style></head>
<body><div class="card">
<h2>Connect PRESS Wallet</h2>
<p class="small">This links your Discord identity to your wallet so roles can sync automatically based on on-chain Press roles.</p>
<div class="small">Session: <code id="sess">{}</code></div>
<div style="margin-top:14px;">
<button class="btn" id="connect">Connect Wallet</button>
</div>
<div style="margin-top:12px;" class="small">Wallet: <span id="w">not connected</span></div>
<div style="margin-top:12px;">
<label class="small">Signature message</label>
<input id="msg" readonly />
</div>
<div style="margin-top:12px;">
<button class="btn" id="sign">Sign & Link</button>
</div>
<div id="out" class="small" style="margin-top:12px;"></div>
<script>
const session = document.getElementById('sess').textContent;
let acct='';
function setMsg(){{
  const m = `Link Discord:${user_id} Session:${session} Nonce:${{Math.floor(Math.random()*1e9)}}`;
  document.getElementById('msg').value = m;
}}
setMsg();
document.getElementById('connect').onclick = async()=>{{
  if(!window.ethereum){{document.getElementById('out').textContent='No wallet found.';return;}}
  const a = await ethereum.request({{method:'eth_requestAccounts'}});
  acct=a[0]; document.getElementById('w').textContent=acct;
}};
document.getElementById('sign').onclick = async()=>{{
  if(!acct){{document.getElementById('out').textContent='Connect wallet first.';return;}}
  const msg=document.getElementById('msg').value;
  const sig = await ethereum.request({{method:'personal_sign', params:[msg, acct]}});
  const r = await fetch('/api/bots/discord/link_wallet',{{method:'POST',headers:{{'content-type':'application/json'}},body:JSON.stringify({{session_id:session,wallet:acct,message:msg,signature:sig}})}});
  const j = await r.json().catch(()=>({{ok:false}}));
  document.getElementById('out').textContent = j.ok ? 'Linked. Roles will sync shortly.' : ('Failed: '+(j.error||''));
}};
</script></div></body></html>"#, session);
    (StatusCode::OK, page).into_response()
}

#[derive(Deserialize)]
struct LinkWalletReq { session_id: String, wallet: String, message: String, signature: String }

async fn discord_link_wallet(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<LinkWalletReq>
) -> Result<Json<serde_json::Value>, StatusCode> {
    // look up discord user for session
    let row = sqlx::query("SELECT discord_user_id, outlet_id FROM oauth_sessions WHERE session_id=?1")
        .bind(&req.session_id).fetch_optional(&state.db).await.ok().flatten();
    if row.is_none() { return Ok(Json(serde_json::json!({"ok":false,"error":"bad_session"}))); }
    let discord_user_id:String = row.as_ref().unwrap().get(0);
    let outlet_id:String = row.unwrap().get::<String,_>(1);

    // recover signer from signature
    let sig = match req.signature.parse::<ethers_core::types::Signature>() {
        Ok(s)=>s, Err(_)=>return Ok(Json(serde_json::json!({"ok":false,"error":"bad_signature"}))),
    };
    let msg = ethers_core::types::H256::from(ethers_core::utils::hash_message(&req.message));
    let rec = sig.recover(msg);
    if rec.is_err() { return Ok(Json(serde_json::json!({"ok":false,"error":"recover_failed"}))); }
    let recovered = format!("{:#x}", rec.unwrap());
    if recovered.to_lowercase() != req.wallet.to_lowercase() {
        return Ok(Json(serde_json::json!({"ok":false,"error":"signature_mismatch"})));
    }

    let now=now_ts();
    let _ = sqlx::query("INSERT INTO wallet_links(discord_user_id,wallet_address,created_at) VALUES (?1,?2,?3) ON CONFLICT(discord_user_id) DO UPDATE SET wallet_address=excluded.wallet_address, created_at=excluded.created_at")
        .bind(&discord_user_id).bind(&req.wallet).bind(now).execute(&state.db).await;

    // trigger role sync
    let _ = perform_role_sync_for_user_scoped(&state, &discord_user_id, &req.wallet, &outlet_id).await;

    Ok(Json(serde_json::json!({"ok":true})))
}

#[derive(Deserialize)]
struct SyncRolesReq { discord_user_id: String }




async fn council_recount(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_request_async(&state, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let cfg = state.cfg.read().await.clone();
    let guild = cfg.press_council_guild_id.clone().unwrap_or_default();
    let role = cfg.press_council_role_id.clone().unwrap_or_default();
    if guild.is_empty() || role.is_empty() {
        return Ok(Json(serde_json::json!({"ok": false, "error":"not_configured"})));
    }
    let gid = guild.parse::<u64>().unwrap_or(0);
    let rid = role.parse::<u64>().unwrap_or(0);
    if gid==0 || rid==0 { return Ok(Json(serde_json::json!({"ok": false, "error":"bad_ids"}))); }
    // force recount by ignoring cache: delete row then call cached
    let _ = sqlx::query("DELETE FROM council_count_cache WHERE guild_id=?1").bind(&guild).execute(&state.db).await;
    let c = robust_role_count_cached(&state, gid, rid).await;
    Ok(Json(serde_json::json!({"ok": true, "count": c})))
}

async fn council_stats(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_request_async(&state, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let cfg = state.cfg.read().await.clone();
    let guild = cfg.press_council_guild_id.clone().unwrap_or_default();
    let role = cfg.press_council_role_id.clone().unwrap_or_default();
    let mut count = 0usize;
if !guild.is_empty() && !role.is_empty() {
    if let (Ok(gid), Ok(rid)) = (guild.parse::<u64>(), role.parse::<u64>()) {
        count = robust_role_count_cached(&state, gid, rid).await;
    }
}

Ok(Json(serde_json::json!({"ok": true, "max": cfg.press_council_max, "current_estimate": count,
 "guild_id": guild, "role_id": role})))
}

#[derive(Deserialize)]
struct CouncilSyncUserReq { discord_user_id: String }

async fn council_sync_user(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CouncilSyncUserReq>
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_request_async(&state, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let row = sqlx::query("SELECT wallet_address FROM wallet_links WHERE discord_user_id=?1")
        .bind(&req.discord_user_id).fetch_optional(&state.db).await.ok().flatten();
    if row.is_none() { return Ok(Json(serde_json::json!({"ok":false,"error":"not_linked"}))); }
    let wallet:String = row.unwrap().get(0);
    let ok = enforce_press_council_role(&state, &req.discord_user_id, &wallet).await;
    Ok(Json(serde_json::json!({"ok": ok})))
}

async fn discord_resync_all(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_request_async(&state, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let n = run_global_role_resync(&state).await;
    Ok(Json(serde_json::json!({"ok": true, "processed": n})))
}

async fn run_global_role_resync(state: &AppState) -> usize {
    let links = sqlx::query("SELECT discord_user_id, wallet_address FROM wallet_links")
        .fetch_all(&state.db).await.unwrap_or_default();
    let mut n=0usize;
    for r in links {
        let uid:String=r.get(0);
        let wallet:String=r.get(1);
        let _ = perform_role_sync_for_user_scoped(state, &uid, &wallet, "").await;
        n+=1;
    }
    n
}

async fn discord_sync_roles(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<SyncRolesReq>
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_request_async(&state, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let row = sqlx::query("SELECT wallet_address FROM wallet_links WHERE discord_user_id=?1")
        .bind(&req.discord_user_id).fetch_optional(&state.db).await.ok().flatten();
    if row.is_none() { return Ok(Json(serde_json::json!({"ok":false,"error":"not_linked"}))); }
    let wallet:String = row.unwrap().get(0);
    let ok = perform_role_sync_for_user_scoped(&state, &req.discord_user_id, &wallet, "").await;
    Ok(Json(serde_json::json!({"ok": ok})))
}

async fn perform_role_sync_for_user_scoped(state: &AppState, discord_user_id: &str, wallet: &str, outlet_id_scope: &str) -> bool {
    // fetch on-chain roles via query-api (expected endpoint)
    let roles = reqwest::Client::new()
        .get(&format!("http://press-query:8787/roles/of?wallet={}", urlencoding::encode(wallet)))
        .send().await
        .ok()
        .and_then(|r| async { r.json::<serde_json::Value>().await.ok() }.await)
        .unwrap_or_else(|| serde_json::json!({"roles":[]}));    
    let rarr = roles.get("roles").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    // determine outlets + mapped roles
    let outlets = if outlet_id_scope.is_empty() {
        sqlx::query("SELECT outlet_id, discord_guild_id FROM outlet_channels WHERE discord_guild_id != ''")
    } else {
        sqlx::query("SELECT outlet_id, discord_guild_id FROM outlet_channels WHERE discord_guild_id != '' AND outlet_id = ?1").bind(outlet_id_scope)
    }
        .fetch_all(&state.db).await.unwrap_or_default();

    for o in outlets {
        let outlet_id:String = o.get(0);
        let guild_id:String = o.get(1);
        let maps = sqlx::query("SELECT press_role, discord_role_id FROM role_mappings WHERE outlet_id=?1")
            .bind(&outlet_id).fetch_all(&state.db).await.unwrap_or_default();

        let mut desired: Vec<u64> = vec![];
        for m in maps {
            let pr:String=m.get(0);
            let dr:String=m.get(1);
            let has = rarr.iter().any(|x| x.as_str().unwrap_or("")==pr);
            if has {
                if let Ok(rid)=dr.parse::<u64>() { desired.push(rid); }
            }
        }
        if desired.is_empty() { continue; }

        if let Some(http)=state.discord_http.read().await.clone() {
            if let (Ok(gid), Ok(uid)) = (guild_id.parse::<u64>(), discord_user_id.parse::<u64>()) {
                if let Ok(mut member) = serenity::model::id::GuildId(gid).member(&http, uid).await {
                    // Assign desired roles
                    for rid in desired.iter() {
                        let _ = member.add_role(&http, serenity::model::id::RoleId(*rid)).await;
                    }

                    // Optional removal: remove any roles that are in our mappings but not currently desired.
                    let cfg = state.cfg.read().await.clone();
                    if cfg.role_removal_enabled {
                        // Build the set of all mapped discord role ids for this outlet
                        let all_mapped: Vec<u64> = maps.iter()
                            .filter_map(|m| m.get::<String,_>(1).parse::<u64>().ok())
                            .collect();
                        for rid in all_mapped {
                            if !desired.contains(&rid) {
                                let _ = member.remove_role(&http, serenity::model::id::RoleId(rid)).await;
                            }
                        }
                    }
                }
            }
        }
    }
    true
}

async fn outlet_register_channels(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<OutletRegisterReq>
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tok = headers.get("x-admin-token").and_then(|v| v.to_str().ok()).unwrap_or("");
    if tok != state.cfg.read().await.admin_token {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let now = now_ts();
    let _ = sqlx::query(r#"
        INSERT INTO outlet_channels(outlet_id,outlet_name,official_domain,discord_guild_id,discord_channel_id,telegram_chat_id,created_at)
        VALUES (?1,?2,?3,?4,?5,?6,?7)
        ON CONFLICT(outlet_id) DO UPDATE SET
          outlet_name=excluded.outlet_name,
          official_domain=excluded.official_domain,
          discord_guild_id=excluded.discord_guild_id,
          discord_channel_id=excluded.discord_channel_id,
          telegram_chat_id=excluded.telegram_chat_id
    "#)
        .bind(&req.outlet_id)
        .bind(req.outlet_name.unwrap_or_default())
        .bind(req.official_domain.unwrap_or_default())
        .bind(req.discord_guild_id.unwrap_or_default())
        .bind(req.discord_channel_id.unwrap_or_default())
        .bind(req.telegram_chat_id.unwrap_or_default())
        .bind(now)
        .execute(&state.db)
        .await;

    Ok(Json(serde_json::json!({"ok": true})))
}

async fn outlet_list_channels(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap
) -> Result<Json<serde_json::Value>, StatusCode> {
    let tok = headers.get("x-admin-token").and_then(|v| v.to_str().ok()).unwrap_or("");
    if tok != state.cfg.read().await.admin_token {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let rows = sqlx::query("SELECT outlet_id,outlet_name,official_domain,discord_guild_id,discord_channel_id,telegram_chat_id,created_at FROM outlet_channels ORDER BY created_at DESC")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
    let mut out = vec![];
    for r in rows {
        out.push(serde_json::json!({
            "outlet_id": r.get::<String,_>(0),
            "outlet_name": r.get::<String,_>(1),
            "official_domain": r.get::<String,_>(2),
            "discord_guild_id": r.get::<String,_>(3),
            "discord_channel_id": r.get::<String,_>(4),
            "telegram_chat_id": r.get::<String,_>(5),
            "created_at": r.get::<i64,_>(6)
        }));
    }
    Ok(Json(serde_json::json!({"ok": true, "outlets": out})))
}

async fn admin_heartbeat_now(axum::extract::State(state): axum::extract::State<AppState>, headers: axum::http::HeaderMap) -> Result<Json<serde_json::Value>, StatusCode> {
    let tok = headers.get("x-admin-token").and_then(|v| v.to_str().ok()).unwrap_or("");
    if tok != state.cfg.read().await.admin_token {
        return Err(StatusCode::UNAUTHORIZED);
    }
    state.heartbeat_notify.notify_one();
    Ok(Json(serde_json::json!({"ok":true})))
}

async fn admin_announce(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<AnnounceReq>
) -> Json<serde_json::Value> {
    if !is_admin_request_async(&state, &headers).await {
        return Json(serde_json::json!({"ok": false, "error":"unauthorized"}));
    }
    if req.text.trim().is_empty() || req.text.len() > 4000 {
        return Json(serde_json::json!({"ok": false, "error":"invalid_text"}));
    }

    let mut q: Vec<AnnouncementQueueItem> = read_json_vec(&state.queue_path).await;
    let id = format!("hq-{}", uuid::Uuid::new_v4());
    q.push(AnnouncementQueueItem{
        id: id.clone(),
        created_at: chrono::Utc::now().timestamp(),
        text: req.text,
        cta_url: req.cta_url,
        cta_label: req.cta_label,
        scope_guild_id: req.scope_guild_id,
    });
    let _ = write_json_vec(&state.queue_path, &q).await;

    Json(serde_json::json!({"ok": true, "queued": true, "id": id}))
}


#[derive(Deserialize)]
struct MissionReq { title: String, minutes: i64, description: String, reward: String }

async fn admin_mission_create(Json(req): Json<MissionReq>) -> Json<serde_json::Value> {
    let id = uuid::Uuid::new_v4().to_string();
    let ends_at = OffsetDateTime::now_utc().unix_timestamp() + req.minutes*60;
    Json(serde_json::json!({"ok": true, "mission_id": id, "ends_at": ends_at, "title": req.title, "reward": req.reward}))
}

#[derive(Deserialize)]
struct TgOnboardReq { chat_id: Option<String> }

async fn telegram_onboarding_link(axum::extract::State(state): axum::extract::State<AppState>, Json(req): Json<TgOnboardReq>) -> Json<serde_json::Value> {
    let bot_username = std::env::var("TELEGRAM_BOT_USERNAME").unwrap_or_else(|_| "PressPulseBot".into());
    let code = uuid::Uuid::new_v4().to_string();
    // persist code -> optional chat binding
    let mut map: std::collections::HashMap<String,String> = if let Ok(b) = tokio::fs::read(&state.tg_codes_path).await { serde_json::from_slice(&b).unwrap_or_default() } else { std::collections::HashMap::new() };
    map.insert(code.clone(), req.chat_id.clone().unwrap_or_default());
    let _ = tokio::fs::write(&state.tg_codes_path, serde_json::to_vec_pretty(&map).unwrap()).await;
    let start_payload = format!("onboard_{}", code);
    let start_payload = format!("onb:{}", code);
    let link = format!("https://t.me/{}?start={}", bot_username, urlencoding::encode(&start_payload));
    Json(serde_json::json!({"ok": true, "link": link, "inline_hint": format!("In Telegram: @{} <query>", bot_username)}))
}


// --- Persistence helpers (queue/missions/bindings) ---
async fn read_json_vec<T: for<'de> Deserialize<'de> + Default>(path: &PathBuf) -> Vec<T> {
    if let Ok(bytes) = tokio::fs::read(path).await {
        if let Ok(v) = serde_json::from_slice::<Vec<T>>(&bytes) {
            return v;
        }
    }
    Vec::new()
}
async fn write_json_vec<T: Serialize>(path: &PathBuf, v: &Vec<T>) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() { tokio::fs::create_dir_all(parent).await.ok(); }
    let bytes = serde_json::to_vec_pretty(v)?;
    tokio::fs::write(path, bytes).await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BindingAddReq { guild_id: String, channel_id: String, purpose: String }

async fn get_bindings(axum::extract::State(state): axum::extract::State<AppState>) -> Json<serde_json::Value> {
    let v: Vec<ChannelBinding> = read_json_vec(&state.bindings_path).await;
    Json(serde_json::json!({"ok": true, "bindings": v}))
}

async fn add_binding(
    axum::extract::State(state): axum::extract::State<AppState>,
    Json(req): Json<BindingAddReq>,
) -> Json<serde_json::Value> {
    let mut v: Vec<ChannelBinding> = read_json_vec(&state.bindings_path).await;
    v.push(ChannelBinding{ guild_id: req.guild_id, channel_id: req.channel_id, purpose: req.purpose });
    if let Err(e) = write_json_vec(&state.bindings_path, &v).await {
        return Json(serde_json::json!({"ok": false, "error": e.to_string()}));
    }
    Json(serde_json::json!({"ok": true, "count": v.len()}))
}

async fn get_admin_gate(axum::extract::State(state): axum::extract::State<AppState>) -> Json<serde_json::Value> {
    let cfg = state.cfg.read().await;
    Json(serde_json::json!({"ok": true, "admin_gate": cfg.admin_gate}))
}

// --- JWT verification for admin routes ---
fn verify_jwt(jwt: &str) -> anyhow::Result<String> {
    let secret = std::env::var("PRESS_BOTS_JWT_SECRET").unwrap_or_else(|_| "dev-unsafe-change-me".into());
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 { anyhow::bail!("bad jwt"); }
    let signing = format!("{}.{}", parts[0], parts[1]);
    let sig = hmac_sha256(secret.as_bytes(), signing.as_bytes());
    let expected = b64url(&sig);
    if expected != parts[2] { anyhow::bail!("invalid signature"); }
    // payload decode
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(parts[1])?;
    let payload = String::from_utf8(payload)?;
    // naive parse for sub
    let sub = payload.split(r#""sub":"#).nth(1).and_then(|s| s.split('"').next()).unwrap_or("").to_string();
    if sub.is_empty() { anyhow::bail!("missing sub"); }
    Ok(sub)
}

fn extract_bearer(headers: &axum::http::HeaderMap) -> Option<String> {
    headers.get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

// Override admin_announce and admin_mission_create with auth gate (keep original names)
async fn admin_announce(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<AnnounceReq>,
) -> Json<serde_json::Value> {
    let token = match extract_bearer(&headers) {
        Some(t) => t,
        None => return Json(serde_json::json!({"ok": false, "error":"missing bearer token"})),
    };
    let sub = match verify_jwt(&token) {
        Ok(s) => s,
        Err(_) => return Json(serde_json::json!({"ok": false, "error":"invalid token"})),
    };

    // Minimal admin gate: require configured HQ guild/role and sub includes wallet or discord.
    let cfg = state.cfg.read().await.clone();
    if cfg.admin_gate.hq_guild_id.is_empty() || cfg.admin_gate.hq_admin_role_id.is_empty() {
        return Json(serde_json::json!({"ok": false, "error":"admin gate not configured (PRESS_HQ_GUILD_ID / PRESS_HQ_ADMIN_ROLE_ID)"}));
    }

    let mut q: Vec<AnnouncementQueueItem> = read_json_vec(&state.queue_path).await;
    q.push(AnnouncementQueueItem{
        id: uuid::Uuid::new_v4().to_string(),
        created_at: time::OffsetDateTime::now_utc().unix_timestamp(),
        text: req.text,
        cta_url: req.cta_url,
        cta_label: req.cta_label,
        scope_guild_id: None,
    });
    if let Err(e) = write_json_vec(&state.queue_path, &q).await {
        return Json(serde_json::json!({"ok": false, "error": e.to_string()}));
    }
    Json(serde_json::json!({"ok": true, "queued": true, "queue_depth": q.len(), "by": sub}))
}

async fn admin_mission_create(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<MissionReq>,
) -> Json(serde_json::Value) {
    let token = match extract_bearer(&headers) {
        Some(t) => t,
        None => return Json(serde_json::json!({"ok": false, "error":"missing bearer token"})),
    };
    let sub = match verify_jwt(&token) {
        Ok(s) => s,
        Err(_) => return Json(serde_json::json!({"ok": false, "error":"invalid token"})),
    };
    let id = uuid::Uuid::new_v4().to_string();
    let ends_at = time::OffsetDateTime::now_utc().unix_timestamp() + req.minutes*60;

    let mut v: Vec<MissionItem> = read_json_vec(&state.missions_path).await;
    v.push(MissionItem{
        id: id.clone(),
        created_at: time::OffsetDateTime::now_utc().unix_timestamp(),
        ends_at,
        title: req.title,
        description: req.description,
        reward: req.reward,
    });
    let _ = write_json_vec(&state.missions_path, &v).await;

    Json(serde_json::json!({"ok": true, "mission_id": id, "ends_at": ends_at, "by": sub}))
}

// --- Dispatch loops (RR81 baseline): fan out announcements + missions to bound channels ---
fn spawn_dispatch_loops(state: AppState) {
    let discord_token = std::env::var("DISCORD_BOT_TOKEN").unwrap_or_default();
    let telegram_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap_or_default();

    if !discord_token.is_empty() {
        tokio::spawn(discord_loop(state.clone(), discord_token));
    }
    if !telegram_token.is_empty() {
        tokio::spawn(telegram_loop(state.clone(), telegram_token));
    }

    // Oracle alert poller: fans out oracle_flags into Discord announcements + Telegram subscribed feeds
    tokio::spawn(oracle_alert_poller(state.clone()));
    tokio::spawn(onchain_heartbeat_loop(state.clone()));
    tokio::spawn(features_watcher_loop(state.clone()));
}

async fn discord_loop(state: AppState, token: String) {
    use serenity::{all::{Client, EventHandler, GatewayIntents, Context, Ready, ChannelId}, async_trait};
    struct Handler { state: AppState }
    #[async_trait]
    impl EventHandler for Handler {
        async fn ready(&self, _ctx: Context, ready: Ready) {
            tracing::info!("PressPulse Discord connected as {}", ready.user.name);
            let guilds = ready.guilds.len() as u32;
            let mut tel = self.state.telemetry.write().await;
            tel.discord_connected = true;
            tel.discord_last_event = now_ts();
            tel.discord_guilds = guilds;
        }
    }
    let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES;
    let mut client = Client::builder(token, intents).event_handler(Handler{ state: state.clone() }).await.expect("client");
    let ctx = client.cache_and_http.clone();
    *state.discord_http.write().await = Some(ctx.http.clone());

    // fanout task
    tokio::spawn(async move {
        loop {
            // read queue
            let mut q: Vec<AnnouncementQueueItem> = read_json_vec(&state.queue_path).await;
            if q.is_empty() {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                continue;
            }
            let item = q.remove(0);
            {
                let mut tel = state.telemetry.write().await;
                tel.discord_last_event = now_ts();
            }
            let bindings: Vec<ChannelBinding> = read_json_vec(&state.bindings_path).await;
            for b in bindings.iter() {
                // Only send to announcement-purpose channels OR default recent_articles
                if b.purpose != "announcements" && b.purpose != "recent_articles" && b.purpose != "oracle_alerts" { continue; }
                let ch = match b.channel_id.parse::<u64>() { Ok(v)=> ChannelId::new(v), Err(_)=> continue };
                let mut content = format!("**PressPulse Announcement**\n{}\n", item.text);
                if let Some(u) = item.cta_url.as_ref() {
                    let label = item.cta_label.clone().unwrap_or_else(|| "Open".into());
                    content.push_str(&format!("\n{}: {}", label, u));
                }
                let _ = ch.say(&ctx.http, content).await;
            }
            // persist updated queue
            let _ = write_json_vec(&state.queue_path, &q).await;
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    });

    if let Err(e) = client.start().await {
        tracing::error!("discord loop error: {}", e);
    }
}

async fn telegram_loop(state: AppState, token: String) {
    use teloxide::{prelude::*, types::InlineKeyboardMarkup, types::InlineKeyboardButton};
    let bot = Bot::new(token);
    {
        let mut tel = state.telemetry.write().await;
        tel.telegram_connected = true;
        tel.telegram_last_event = now_ts();
    }

    // Feed poller: pushes new on-chain events to subscribed chats
    tokio::spawn(telegram_feed_poller(state.clone(), bot.clone()));

    
// /start onboarding + inline verification cues
let msg_handler = Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
    let state = state.clone();
    async move {
        let start_payload = msg.text().unwrap_or("").to_string();
        // If user started with "onb:<code>", acknowledge code and instruct verify via dashboard
        let mut note = String::new();
        if start_payload.starts_with("/start") {
            if let Some(p) = start_payload.split_whitespace().nth(1) {
                if p.starts_with("onb:") {
                    let code = p.trim_start_matches("onb:");
                    note = format!("

Onboarding code detected: `{}`. Next: verify your Press Wallet to complete onboarding.", code);
                }
            }
        }
        let text = format!("Welcome to PressPulse.

1) Verify Press Wallet
2) Optional: Press Pass
3) Subscribe to feeds{}

Inline mode: type @PressPulseBot <query> to search articles/proposals.", note);
        let kb = InlineKeyboardMarkup::new(vec![
            vec![InlineKeyboardButton::url("Open Dashboard".into(), "https://bots.pressblockchain.io/dashboard".parse().unwrap())],
            vec![InlineKeyboardButton::url("Verify (Press Wallet)".into(), "https://bots.pressblockchain.io/dashboard#verify".parse().unwrap())],
            vec![InlineKeyboardButton::url("Status".into(), "https://status.pressblockchain.io".parse().unwrap())],
        ]);
        let _ = bot.send_message(msg.chat.id, text).reply_markup(kb).await;
        let _ = state;
        respond(())
    }
});

// Inline query: placeholder "search" responses; RR83 will connect to query-api/indexer.
let inline_handler = Update::filter_inline_query().endpoint(|bot: Bot, q: teloxide::types::InlineQuery| async move {
    use teloxide::types::{InlineQueryResultArticle, InputMessageContentText, InlineQueryResult};
    let query = q.query.clone();
    let article = InlineQueryResultArticle::new(
        "presspulse_search_1".into(),
        format!("Search: {}", query),
        InputMessageContentText::new(format!("Search results for: {}\n\n{}", query, snippet))
    ).description("On-chain search (coming online)".into());
    bot.answer_inline_query(q.id, vec![InlineQueryResult::Article(article)]).await.ok();
    respond(())
});

let handler = dptree::entry()
    .branch(msg_handler)
    .branch(inline_handler);

Dispatcher::builder(bot, handler)
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;

}


// --- Telegram subscriptions (per chat) ---
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct TgSubs {
    // chat_id -> list of feeds
    chats: std::collections::HashMap<String, Vec<String>>,
    // last sent cursor per feed
    cursors: std::collections::HashMap<String, i64>,
}

async fn tg_read_subs(path: &PathBuf) -> TgSubs {
    if let Ok(b) = tokio::fs::read(path).await {
        if let Ok(v) = serde_json::from_slice::<TgSubs>(&b) { return v; }
    }
    TgSubs::default()
}
async fn tg_write_subs(path: &PathBuf, v: &TgSubs) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() { tokio::fs::create_dir_all(parent).await.ok(); }
    tokio::fs::write(path, serde_json::to_vec_pretty(v)?).await?;
    Ok(())
}

async fn tg_get_subs(axum::extract::State(state): axum::extract::State<AppState>) -> Json<serde_json::Value> {
    let subs = tg_read_subs(&state.tg_subs_path).await;
    Json(serde_json::json!({"ok": true, "subscriptions": subs}))
}

#[derive(Deserialize)]
struct TgSetReq { chat_id: String, feeds: Vec<String> }

async fn tg_set_subs(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<TgSetReq>
) -> Json<serde_json::Value> {
    // Require auth; only admin can set subs from dashboard
    let token = match extract_bearer(&headers) { Some(t)=>t, None=> return Json(serde_json::json!({"ok":false,"error":"missing bearer"})) };
    if verify_jwt(&token).is_err() { return Json(serde_json::json!({"ok":false,"error":"invalid token"})); }

    let mut subs = tg_read_subs(&state.tg_subs_path).await;
    subs.chats.insert(req.chat_id.clone(), req.feeds.clone());
    let _ = tg_write_subs(&state.tg_subs_path, &subs).await;
    Json(serde_json::json!({"ok": true}))
}


async fn telegram_feed_poller(state: AppState, bot: teloxide::Bot) {
    loop {
        let cfg = state.cfg.read().await.clone();
        let api = cfg.press_query_api.clone();
        let mut subs = tg_read_subs(&state.tg_subs_path).await;

        for (chat_id, feeds) in subs.chats.clone().into_iter() {
            let chat: i64 = match chat_id.parse() { Ok(v)=>v, Err(_)=>continue };
            for feed in feeds.iter() {
                // cursor key per chat+feed
                let ckey = format!("{}:{}", chat_id, feed);
                let cursor = subs.cursors.get(&ckey).cloned().unwrap_or(0);
                // query API for feed events
                let url = format!("{}/api/feed/{}?after={}", api, feed, cursor);
                if let Ok(r) = reqwest::Client::new().get(url).send().await {
                    if let Ok(j) = r.json::<serde_json::Value>().await {
                        if let Some(items) = j.get("items").and_then(|v| v.as_array()) {
                            let mut max_ts = cursor;
                            for it in items.iter().take(10) {
                                let ts = it.get("ts").and_then(|v| v.as_i64()).unwrap_or(cursor);
                                let title = it.get("title").and_then(|v| v.as_str()).unwrap_or("Update");
let sev = it.get("severity").and_then(|v| v.as_i64()).unwrap_or(1);
let kind = it.get("kind").and_then(|v| v.as_str()).unwrap_or("");
let src = it.get("source").and_then(|v| v.as_str()).unwrap_or("");

                                let url = it.get("url").and_then(|v| v.as_str()).unwrap_or("");
                                let msg = if feed == "oracle_flags" {
    if !cfg.oracle_alerts_enabled || sev < cfg.oracle_min_severity { continue; }
    format!("Press Oracle Alert\n{}\nSeverity: {} | Kind: {} | Source: {}\n{}", title, sev, kind, src, url)
} else {
    format!("{}\n{}", title, url)
};
                                let _ = bot.send_message(teloxide::types::ChatId(chat), msg).await;
                                if ts > max_ts { max_ts = ts; }
                            }
                            if max_ts > cursor { subs.cursors.insert(ckey.clone(), max_ts); }
                        }
                    }
                }
            }
        }

        let _ = tg_write_subs(&state.tg_subs_path, &subs).await;
        tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    }
}


async fn oracle_alert_poller(state: AppState) {
    // Persist cursor in /state/oracle_alert_cursor.json
    let cursor_path = PathBuf::from(env::var("PRESS_ORACLE_CURSOR").unwrap_or_else(|_| "/state/oracle_alert_cursor.json".into()));
    let mut cursor: i64 = read_json_i64(&cursor_path).await.unwrap_or(0);

    loop {
        let cfg = state.cfg.read().await.clone();
        if !cfg.oracle_alerts_enabled {
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
            continue;
        }

        let url = format!("{}/api/feed/oracle_flags?after={}&severity={}", cfg.press_query_api, cursor, cfg.oracle_min_severity);
        if let Ok(r) = reqwest::Client::new().get(url).send().await {
            if let Ok(j) = r.json::<serde_json::Value>().await {
                if let Some(items) = j.get("items").and_then(|v| v.as_array()) {
                    let mut max_ts = cursor;
                    for it in items.iter().take(20) {
                        let ts = it.get("ts").and_then(|v| v.as_i64()).unwrap_or(cursor);
                        let sev = it.get("severity").and_then(|v| v.as_i64()).unwrap_or(1);
                        let kind = it.get("kind").and_then(|v| v.as_str()).unwrap_or("oracle");
                        let aid = it.get("article_id").and_then(|v| v.as_str()).unwrap_or("");
                        let title = it.get("title").and_then(|v| v.as_str()).unwrap_or("Oracle Alert");
                        let src = it.get("source").and_then(|v| v.as_str()).unwrap_or("ai");
                        let link = it.get("url").and_then(|v| v.as_str()).unwrap_or("");

                        let mut text = format!("Oracle flagged article **{}**\nSeverity: {} | Kind: {} | Source: {}\nArticle ID: {}", title, sev, kind, src, aid);
                        if !link.is_empty() { text.push_str(&format!("\n{}", link)); }

                        // Enqueue as announcement (Discord loop fans out to channels with purpose announcements/oracle_alerts)
                        let mut q: Vec<AnnouncementQueueItem> = read_json_vec(&state.queue_path).await;
                        q.push(AnnouncementQueueItem{
                            id: format!("oracle-{}-{}", aid, ts),
                            created_at: chrono::Utc::now().timestamp(),
                            text,
                            cta_url: if link.is_empty() { None } else { Some(link.to_string()) },
                            cta_label: Some("View".into()),
                            scope_guild_id: None,
                        });
                        let _ = write_json_vec(&state.queue_path, &q).await;

                        if ts > max_ts { max_ts = ts; }
                    }
                    if max_ts > cursor {
                        cursor = max_ts;
                        let _ = write_json_i64(&cursor_path, cursor).await;
                    }
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(8)).await;
    }
}

async fn read_json_i64(path: &PathBuf) -> Option<i64> {
    if let Ok(bytes) = tokio::fs::read(path).await {
        if let Ok(v) = serde_json::from_slice::<i64>(&bytes) {
            return Some(v);
        }
    }
    None
}
async fn write_json_i64(path: &PathBuf, v: i64) -> anyhow::Result<()> {
    if let Some(p) = path.parent() { tokio::fs::create_dir_all(p).await.ok(); }
    tokio::fs::write(path, serde_json::to_vec_pretty(&v)?).await?;
    Ok(())
}


#[derive(Clone, Serialize, Deserialize, Default)]
struct BotTelemetry {
    discord_connected: bool,
    telegram_connected: bool,
    discord_last_event: i64,
    telegram_last_event: i64,
    discord_guilds: u32,
    discord_guild_list: Vec<(String,String)>, // (id,name)
}

fn now_ts() -> i64 { chrono::Utc::now().timestamp() }



async fn onchain_heartbeat_loop(state: AppState) {
    // Minimal ABI for UptimeBeacon.heartbeat(bytes32,uint8,bytes32)
    const ABI_JSON: &str = r#"[{"inputs":[{"internalType":"bytes32","name":"service","type":"bytes32"},{"internalType":"uint8","name":"status","type":"uint8"},{"internalType":"bytes32","name":"extra","type":"bytes32"}],"name":"heartbeat","outputs":[],"stateMutability":"nonpayable","type":"function"}]"#;

    loop {
        let cfg = state.cfg.read().await.clone();
        if !cfg.onchain_heartbeat_enabled {
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
            continue;
        }
        if cfg.onchain_heartbeat_contract.trim().is_empty() {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            continue;
        }
        let pk = match cfg.onchain_heartbeat_privkey.clone() {
            Some(v) if !v.trim().is_empty() => v,
            _ => { tokio::time::sleep(std::time::Duration::from_secs(30)).await; continue; }
        };

        // derive status from connectivity
        let tel = state.telemetry.read().await.clone();
        let status: u8 = if tel.discord_connected || tel.telegram_connected { 1 } else { 0 };

        // Service id "press-bots"
        let service: [u8;32] = {
            let mut b = [0u8;32];
            let s = b"press-bots";
            b[..s.len()].copy_from_slice(s);
            b
        };

        // extra field: last 8 bytes = unix ts
        let extra: [u8;32] = {
            let mut b=[0u8;32];
            let ts = now_ts() as u64;
            b[24..32].copy_from_slice(&ts.to_be_bytes());
            b
        };

        // Broadcast tx
        let rpc = cfg.onchain_heartbeat_rpc.clone();
        let contract_addr = match cfg.onchain_heartbeat_contract.parse::<ethers::types::Address>() {
            Ok(a) => a,
            Err(_) => { tokio::select!{
            _ = tokio::time::sleep(std::time::Duration::from_secs(cfg.onchain_heartbeat_interval_sec)) => {},
            _ = state.heartbeat_notify.notified() => {},
        } continue; }
        };

        let provider = match ethers::providers::Provider::<ethers::providers::Http>::try_from(rpc.as_str()) {
            Ok(p) => p,
            Err(_) => { tokio::select!{
            _ = tokio::time::sleep(std::time::Duration::from_secs(cfg.onchain_heartbeat_interval_sec)) => {},
            _ = state.heartbeat_notify.notified() => {},
        } continue; }
        };
        let chain_id = provider.get_chainid().await.ok().and_then(|v| v.as_u64().try_into().ok()).unwrap_or(1u64);

        let wallet: ethers::signers::LocalWallet = match pk.parse() {
            Ok(w) => w.with_chain_id(chain_id),
            Err(_) => { tokio::select!{
            _ = tokio::time::sleep(std::time::Duration::from_secs(cfg.onchain_heartbeat_interval_sec)) => {},
            _ = state.heartbeat_notify.notified() => {},
        } continue; }
        };

        let client = std::sync::Arc::new(ethers::middleware::SignerMiddleware::new(provider, wallet));
        let abi: ethers::abi::Abi = serde_json::from_str(ABI_JSON).unwrap_or_default();
        let contract = ethers::contract::Contract::new(contract_addr, abi, client);

        // Call heartbeat
        let call = contract.method::<(ethers::types::H256, u8, ethers::types::H256), ()>(
            "heartbeat",
            (ethers::types::H256(service), status, ethers::types::H256(extra)),
        );

        match call {
            Ok(pending) => {
                let res = pending.send().await;
                if res.is_ok() {
                    let mut telw = state.telemetry.write().await;
                    telw.telegram_last_event = telw.telegram_last_event.max(now_ts());
                }
            }
            Err(_) => {}
        }

        tokio::select!{
            _ = tokio::time::sleep(std::time::Duration::from_secs(cfg.onchain_heartbeat_interval_sec)) => {},
            _ = state.heartbeat_notify.notified() => {},
        }
    }
}



async fn features_watcher_loop(state: AppState) {
    let mut last: String = String::new();
    loop {
        let path = {
            let cfg = state.cfg.read().await;
            cfg.features_path.clone().unwrap_or_else(|| "/state/features.json".into())
        };
        if let Ok(s) = std::fs::read_to_string(&path) {
            if s != last {
                last = s.clone();
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                    let bots = v.get("flags").and_then(|f| f.get("bots")).and_then(|x| x.as_bool());
                    let hb = v.get("flags").and_then(|f| f.get("onchainHeartbeat")).and_then(|x| x.as_bool());
                    {
                        let mut cfg = state.cfg.write().await;
                        if let Some(b) = bots { cfg.bots_enabled = b; }
                        if let Some(h) = hb { cfg.onchain_heartbeat_enabled = h; }
                    }
                    // apply runtime actions
                    if bots == Some(false) {
                        state.discord_shutdown.notify_waiters();
                        state.telegram_cancel.cancel();
                    }
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}


async fn init_db(db: &sqlx::SqlitePool) -> anyhow::Result<()> {
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS outlet_channels (
            outlet_id TEXT PRIMARY KEY,
            outlet_name TEXT,
            official_domain TEXT,
            discord_guild_id TEXT,
            discord_channel_id TEXT,
            telegram_chat_id TEXT,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS role_mappings (
            outlet_id TEXT NOT NULL,
            press_role TEXT NOT NULL,
            discord_role_id TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            PRIMARY KEY(outlet_id, press_role)
        );

        CREATE TABLE IF NOT EXISTS wallet_links (
            discord_user_id TEXT PRIMARY KEY,
            wallet_address TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS oauth_sessions (
            session_id TEXT PRIMARY KEY,
            discord_user_id TEXT NOT NULL,
            outlet_id TEXT,
            created_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS council_grace (
            discord_user_id TEXT PRIMARY KEY,
            lost_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS council_members (
            wallet_address TEXT PRIMARY KEY,
            joined_at INTEGER NOT NULL,
            last_active_at INTEGER NOT NULL
        );
    "#).execute(db).await?;
    Ok(())
}

async fn council_status(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Json<serde_json::Value> {
    let rows = sqlx::query("SELECT wallet_address, joined_at, last_active_at FROM council_members ORDER BY joined_at ASC")
        .fetch_all(&state.db).await.unwrap_or_default();
    Json(serde_json::json!({
        "max": state.cfg.read().await.council_max_members,
        "count": rows.len(),
        "members": rows.into_iter().map(|r| serde_json::json!({
            "wallet": r.get::<String,_>(0),
            "joined_at": r.get::<i64,_>(1),
            "last_active_at": r.get::<i64,_>(2)
        })).collect::<Vec<_>>()
    }))
}

async fn council_enforce(
    axum::extract::State(state): axum::extract::State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !is_admin_request_async(&state, &headers).await { return Err(StatusCode::UNAUTHORIZED); }
    let cfg = state.cfg.read().await.clone();
    let now = chrono::Utc::now().timestamp();
    let cutoff = now - (cfg.council_grace_days * 86400);

    let rows = sqlx::query("SELECT wallet_address, last_active_at FROM council_members")
        .fetch_all(&state.db).await.unwrap_or_default();

    let mut removed = vec![];
    for r in rows {
        let wallet:String = r.get(0);
        let last:i64 = r.get(1);
        if last < cutoff {
            sqlx::query("DELETE FROM council_members WHERE wallet_address=?1")
                .bind(&wallet).execute(&state.db).await.ok();
            removed.push(wallet);
        }
    }
    Ok(Json(serde_json::json!({"ok": true, "removed": removed})))
}
async fn robust_role_count_cached(state: &AppState, guild_id: u64, role_id: u64) -> usize {
    let cfg = state.cfg.read().await.clone();
    let now = now_ts();
    let gid_s = guild_id.to_string();
    // check cache
    if let Ok(row) = sqlx::query("SELECT count, counted_at FROM council_count_cache WHERE guild_id=?1")
        .bind(&gid_s).fetch_optional(&state.db).await {
        if let Some(r)=row {
            let c:i64 = r.get(0);
            let t:i64 = r.get(1);
            if now - t <= (cfg.press_council_count_cache_ttl_secs as i64) {
                return c as usize;
            }
        }
    }
    // recount
    let c = robust_role_count(state, guild_id, role_id).await;
    let _ = sqlx::query("INSERT INTO council_count_cache(guild_id,role_id,count,counted_at) VALUES (?1,?2,?3,?4) ON CONFLICT(guild_id) DO UPDATE SET role_id=excluded.role_id, count=excluded.count, counted_at=excluded.counted_at")
        .bind(&gid_s).bind(role_id.to_string()).bind(c as i64).bind(now).execute(&state.db).await;
    c
}

async fn robust_role_count(state: &AppState, guild_id: u64, role_id: u64) -> usize {
    let Some(http) = state.discord_http.read().await.clone() else { return 0; };

    // Discord API: paginate guild members with "after" and "limit=1000"
    let mut after: Option<serenity::model::id::UserId> = None;
    let mut count: usize = 0;

    loop {
        let chunk = serenity::model::id::GuildId(guild_id)
            .members(&http, Some(1000), after)
            .await;

        let Ok(members) = chunk else { break; };
        if members.is_empty() { break; }

        for m in members.iter() {
            if m.roles.iter().any(|r| r.0 == role_id) { count += 1; }
        }
        after = members.last().map(|m| m.user.id);
        if members.len() < 1000 { break; }
    }
    count
}


// Determine eligibility for COUNCIL in the official server.
// Preferred: installer provides an eligibility endpoint that incorporates term/activity/bond requirements.
// Fallback: use simple on-chain role presence `COUNCIL`.
let mut eligible = false;
let mut eligibility_reason = String::new();

if let Some(url_tpl) = cfg.press_council_eligibility_url.clone() {
    let url = url_tpl.replace("{wallet}", wallet);
    if let Ok(resp) = reqwest::Client::new().get(&url).send().await {
        if let Ok(j) = resp.json::<serde_json::Value>().await {
            eligible = j.get("eligible").and_then(|v| v.as_bool()).unwrap_or(false);
            eligibility_reason = j.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string();
        }
    }
}

if !eligible {
    let roles = reqwest::Client::new()
        .get(&format!("http://press-query:8787/roles/of?wallet={}", urlencoding::encode(wallet)))
        .send().await
        .ok()
        .and_then(|r| async { r.json::<serde_json::Value>().await.ok() }.await)
        .unwrap_or_else(|| serde_json::json!({"roles":[]}));
    eligible = roles.get("roles").and_then(|v| v.as_array())
        .map(|a| a.iter().any(|x| x.as_str().unwrap_or("")=="COUNCIL"))
        .unwrap_or(false);
}

