static REQUESTS: AtomicU64 = AtomicU64::new(0);
static ERRORS: AtomicU64 = AtomicU64::new(0);




async fn registry_contracts() -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let path = std::env::var("STATE_DIR").unwrap_or_else(|_| "/state".into());
    let p = format!("{}/contract_addresses.json", path);
    let data = fs::read_to_string(&p).map_err(|_| StatusCode::NOT_FOUND)?;
    let v: serde_json::Value = serde_json::from_str(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(axum::Json(v))
}



#[derive(serde::Serialize)]
struct ChainMetadata {
    chainId: u64,
    chainName: String,
    rpcUrls: Vec<String>,
    wsUrls: Vec<String>,
    nativeCurrency: serde_json::Value,
    blockExplorerUrls: Vec<String>,
    iconUrls: Vec<String>,
}



async fn deploy_manifest() -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let path = std::env::var("STATE_DIR").unwrap_or_else(|_| "/state".into());
    let p = format!("{}/deploy_manifest.json", path);
    let data = fs::read_to_string(&p).map_err(|_| StatusCode::NOT_FOUND)?;
    let v: serde_json::Value = serde_json::from_str(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(axum::Json(v))
}
async fn chain_metadata() -> axum::Json<ChainMetadata> {
    let chain_id: u64 = std::env::var("CHAIN_ID").ok().and_then(|v| v.parse().ok()).unwrap_or(271828);
    let chain_name = std::env::var("CHAIN_NAME").unwrap_or_else(|_| "Press Blockchain".into());
    let rpc_http = std::env::var("PUBLIC_RPC_HTTP").unwrap_or_else(|_| "https://rpc.pressblockchain.io".into());
    let rpc_wss = std::env::var("PUBLIC_RPC_WSS").unwrap_or_else(|_| "wss://rpc.pressblockchain.io/ws".into());
    let symbol = std::env::var("NATIVE_SYMBOL").unwrap_or_else(|_| "PRESS".into());
    let explorer = std::env::var("EXPLORER_URL").unwrap_or_else(|_| "https://explorer.pressblockchain.io".into());

    let native = serde_json::json!({
        "name": "Press Token",
        "symbol": symbol,
        "decimals": 18
    });

    axum::Json(ChainMetadata{
        chainId: chain_id,
        chainName: chain_name,
        rpcUrls: vec![rpc_http.clone()],
        wsUrls: vec![rpc_wss.clone()],
        nativeCurrency: native,
        blockExplorerUrls: vec![explorer],
        iconUrls: vec![]
    })
}
async fn registry_abi(axum::extract::Path(name): axum::extract::Path<String>) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let path = std::env::var("STATE_DIR").unwrap_or_else(|_| "/state".into());
    let p = format!("{}/abi/{}.json", path, name);
    let data = fs::read_to_string(&p).map_err(|_| StatusCode::NOT_FOUND)?;
    let v: serde_json::Value = serde_json::from_str(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(axum::Json(v))
}
fn module_enabled(key: &str) -> bool {
    let env_key = format!("MODULE_{}", key.to_uppercase());
    std::env::var(env_key).unwrap_or_else(|_| "1".into()) != "0"
}

use axum::{routing::{get, post}, Json, Router, extract::State};
use serde::{Deserialize, Serialize};
use std::{sync::atomic::{AtomicU64, Ordering}, net::SocketAddr, path::PathBuf, fs};
use tower_http::cors::{CorsLayer, Any};
use tracing::info;
use ethers::{prelude::*, types::U256};

static VERSION: &str = "RR109";
const STANDARD_OUTLET_TOKEN_SUPPLY: u128 = 1_000_000_000_000000000000000000u128; // 1e9 * 1e18


#[derive(Clone)]
struct AppState {
    state_dir: PathBuf,
    rpc_url: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]


async fn read_json_file(rel: &str) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    let base = std::env::var("PRESS_REPO_DIR").unwrap_or_else(|_| "/opt/pressblockchain".into());
    let p = PathBuf::from(base).join(rel);
    let data = fs::read_to_string(&p).map_err(|_| StatusCode::NOT_FOUND)?;
    let v: serde_json::Value = serde_json::from_str(&data).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(axum::Json(v))
}

async fn config_modules() -> Result<axum::Json<serde_json::Value>, StatusCode> {
    read_json_file("config/modules.json").await
}
async fn config_listing_tiers() -> Result<axum::Json<serde_json::Value>, StatusCode> {
    read_json_file("apps/shared-config/listing-tiers.json").await
}
async fn config_brand() -> Result<axum::Json<serde_json::Value>, StatusCode> {
    read_json_file("apps/shared-config/press-brand.json").await
}

async fn config_release_presets() -> Result<axum::Json<serde_json::Value>, StatusCode> {
    read_json_file("apps/shared-config/release-presets.json").await
}



async fn tcp_check(host: &str, port: u16) -> bool {
    tokio::task::spawn_blocking(move || std::net::TcpStream::connect((host, port)).is_ok())
        .await
        .unwrap_or(false)
}

async fn health() -> axum::Json<serde_json::Value> {
    // Conservative defaults; installer can override via env.
    let rpc_host = std::env::var("PRESS_RPC_HOST").unwrap_or_else(|_| "rpc.pressblockchain.io".into());
    let rpc_port: u16 = std::env::var("PRESS_RPC_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(8545);

    let bots_host = std::env::var("PRESS_BOTS_HOST").unwrap_or_else(|_| "bots.pressblockchain.io".into());
    let bots_port: u16 = std::env::var("PRESS_BOTS_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(8080);

    let idx_host = std::env::var("PRESS_INDEXER_HOST").unwrap_or_else(|_| "indexer.pressblockchain.io".into());
    let idx_port: u16 = std::env::var("PRESS_INDEXER_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(7400);

    let gateway_port: u16 = std::env::var("PRESS_GATEWAY_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(8085);

    let rpc_ok = tcp_check(&rpc_host, rpc_port).await;
    let bots_ok = tcp_check(&bots_host, bots_port).await;
    let idx_ok = tcp_check(&idx_host, idx_port).await;

    let now = chrono::Utc::now().to_rfc3339();
    axum::Json(serde_json::json!({
        "time": now,
        "gateway": { "ok": true, "port": gateway_port },
        "rpc": { "host": rpc_host, "port": rpc_port, "ok": rpc_ok },
        "bots": { "host": bots_host, "port": bots_port, "ok": bots_ok },
        "indexer": { "host": idx_host, "port": idx_port, "ok": idx_ok }
    }))
}

async fn deploy_snapshot() -> Result<axum::Json<serde_json::Value>, StatusCode> {
    // Installer writes state to `state/deploy_snapshot.json`; gateway serves it read-only.
    read_json_file("state/deploy_snapshot.json").await
}


fn installer_authorized(headers: &axum::http::HeaderMap) -> bool {
    let expected = std::env::var("PRESS_INSTALLER_ADMIN_TOKEN").ok();
    if expected.is_none() {
        // If not set, disable write/exec endpoints for safety.
        return false;
    }
    let expected = expected.unwrap();
    let got = headers.get("x-installer-token").and_then(|v| v.to_str().ok()).unwrap_or("");
    got == expected
}

async fn write_deploy_snapshot(headers: axum::http::HeaderMap, axum::Json(payload): axum::Json<serde_json::Value>)
    -> Result<axum::Json<serde_json::Value>, StatusCode>
{
    if !installer_authorized(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let base = std::env::var("PRESS_REPO_DIR").unwrap_or_else(|_| "/opt/pressblockchain".into());
    let p = PathBuf::from(base).join("state/deploy_snapshot.json");
    if let Some(parent) = p.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let txt = serde_json::to_string_pretty(&payload).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tokio::fs::write(&p, txt).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(axum::Json(serde_json::json!({"ok": true, "path": p.to_string_lossy()})))
}


async fn write_config_modules(headers: axum::http::HeaderMap, axum::Json(payload): axum::Json<serde_json::Value>)
    -> Result<axum::Json<serde_json::Value>, StatusCode>
{
    if !installer_authorized(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    // Expect payload: { "modules": [ { "id": "...", "enabled": true }, ... ] }
    let modules = payload.get("modules").and_then(|v| v.as_array()).ok_or(StatusCode::BAD_REQUEST)?;

    // Load dependency rules from repo file (best-effort). If missing, still allow write.
    let deps_v = read_json_file("config/module_dependencies.json").await.ok().map(|j| j.0).unwrap_or(serde_json::json!({}));

    // Build map for validation
    let mut map: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    for m in modules {
        let id = m.get("id").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;
        let en = m.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
        map.insert(id.to_string(), en);
    }

    // Enforce: cannot disable core_chain
    if let Some(v) = map.get("core_chain") {
        if !*v { return Err(StatusCode::BAD_REQUEST); }
    }

    // Enforce dependencies: if module enabled and depends_on disabled -> reject
    if let Some(obj) = deps_v.as_object() {
        for (mid, rule) in obj.iter() {
            let en = *map.get(mid).unwrap_or(&true);
            if en {
                let depends = rule.get("depends_on").and_then(|v| v.as_array()).cloned().unwrap_or_default();
                for d in depends {
                    if let Some(dep_id) = d.as_str() {
                        if let Some(dep_en) = map.get(dep_id) {
                            if !*dep_en {
                                return Err(StatusCode::BAD_REQUEST);
                            }
                        }
                    }
                }
            }
        }
    }

    // Write modules.json (canonical schema)
    let base = std::env::var("PRESS_REPO_DIR").unwrap_or_else(|_| "/opt/pressblockchain".into());
    let p = PathBuf::from(base).join("config/modules.json");
    if let Some(parent) = p.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    let txt = serde_json::to_string_pretty(&payload).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tokio::fs::write(&p, txt).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(axum::Json(serde_json::json!({"ok": true, "path": p.to_string_lossy()})))
}

async fn run_auto_fix(headers: axum::http::HeaderMap) -> Result<axum::Json<serde_json::Value>, StatusCode> {
    if !installer_authorized(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let base = std::env::var("PRESS_REPO_DIR").unwrap_or_else(|_| "/opt/pressblockchain".into());
    let script = PathBuf::from(base).join("scripts/auto-fix.sh");
    let out = tokio::task::spawn_blocking(move || {
        Command::new("bash").arg(script.to_string_lossy().to_string()).output()
    }).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
      .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(axum::Json(serde_json::json!({
        "ok": out.status.success(),
        "code": out.status.code(),
        "stdout": String::from_utf8_lossy(&out.stdout),
        "stderr": String::from_utf8_lossy(&out.stderr)
    })))
}

struct DeployState {
    #[serde(default)] pressToken: String,
    #[serde(default)] pressParameters: String,
    #[serde(default)] treasury: String,
    #[serde(default)] outletRegistry: String,
    #[serde(default)] outletTokenFactory: String,
    #[serde(default)] exchangeListingRegistry: String,
    #[serde(default)] articleApprovals: String,
}

fn read_deploy_json(state_dir: &PathBuf) -> DeployState {
    let p = state_dir.join("deploy.json");
    if let Ok(s) = fs::read_to_string(&p) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            return DeployState {
                pressToken: v.get("pressToken").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                pressParameters: v.get("pressParameters").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                treasury: v.get("treasury").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                outletRegistry: v.get("outletRegistry").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                outletTokenFactory: v.get("outletTokenFactory").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                exchangeListingRegistry: v.get("exchangeListingRegistry").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
                articleApprovals: v.get("articleApprovals").and_then(|x| x.as_str()).unwrap_or_default().to_string(),
            };
        }
    }
    DeployState::default()
}

fn state_dir() -> PathBuf {
    std::env::var("STATE_DIR").ok().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("./state"))
}

fn rpc_url_from_env() -> String {
    std::env::var("PRESS_RPC_URL").unwrap_or_else(|_| "http://press-rpc:8545".to_string())
}

fn load_owner_pk(st: &AppState, pk_opt: Option<String>) -> Result<String, String> {
    if let Some(pk) = pk_opt {
        let pk = pk.trim();
        if pk.starts_with("0x") { return Ok(pk.to_string()); }
        return Ok(format!("0x{}", pk));
    }
    let p = st.state_dir.join("owner_keys.json");
    if let Ok(s) = fs::read_to_string(&p) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            if let Some(pk) = v.get("deployer_private_key").and_then(|x| x.as_str()) {
                let pk = pk.trim();
                if pk.starts_with("0x") { return Ok(pk.to_string()); }
                return Ok(format!("0x{}", pk));
            }
        }
    }
    if let Ok(pk) = std::env::var("DEPLOYER_PRIVATE_KEY_HEX") {
        let pk = pk.trim();
        if pk.starts_with("0x") { return Ok(pk.to_string()); }
        return Ok(format!("0x{}", pk));
    }
    Err("Missing owner private key. Provide owner_private_key or ensure state/owner_keys.json exists.".to_string())
}

abigen!(
    OutletRegistry,
    r#"[
        function createOutlet(string name,string domain) returns (bytes32)
        function outletIdFromDomain(string domain) view returns (bytes32)
    ]"#,
);

abigen!(
    OutletTokenFactory,
    r#"[
        function deployOutletToken(bytes32 outletId,string n,string s,uint256 mintedSupply) returns (address)
        function outletTokenOf(bytes32 outletId) view returns (address)
    ]"#,
);

abigen!(
    ExchangeListingRegistry,
    r#"[
        function listToken(address token, bytes32 outletId, uint8 tier)
    ]"#,
);

abigen!(
    ERC20Mini,
    r#"[
        function transfer(address to, uint256 amount) returns (bool)
    ]"#,
);

abigen!(
    ArticleApprovals,
    r#"[
        function getCounts(uint256 articleId) view returns (uint64 startAt, uint64 endAt, uint32 community, uint32 outlet, uint32 council, uint32 flags, bool finalized, bool approved)
        function finalize(uint256 articleId)
    ]"#,
);

#[derive(Serialize)]
struct HealthResp { ok: bool, version: &'static str }

async fn health() -> Json<HealthResp> { Json(HealthResp{ ok:true, version: VERSION }) }

#[derive(Serialize)]
struct WizardInfo {
    outlet_registry: String,
    outlet_token_factory: String,
    exchange_listing_registry: String,
    article_approvals: String,
    press_token: String,
    treasury: String,
}

async fn wizard_info(State(st): State<AppState>) -> Json<WizardInfo> {
    let ds = read_deploy_json(&st.state_dir);
    Json(WizardInfo{
        outlet_registry: ds.outletRegistry,
        outlet_token_factory: ds.outletTokenFactory,
        exchange_listing_registry: ds.exchangeListingRegistry,
        article_approvals: ds.articleApprovals,
        press_token: ds.pressToken,
        treasury: ds.treasury,
    })
}

#[derive(Debug, Deserialize)]
struct CreateOutletReq { name: String, domain: String, owner_private_key: Option<String> }

#[derive(Serialize)]
struct CreateOutletResp { ok: bool, outlet_id: String, tx_hash: String }

async fn create_outlet(State(st): State<AppState>, Json(req): Json<CreateOutletReq>) -> Json<CreateOutletResp> {
    let ds = read_deploy_json(&st.state_dir);
    let pk = match load_owner_pk(&st, req.owner_private_key) {
        Ok(pk)=>pk,
        Err(e)=> return Json(CreateOutletResp{ ok:false, outlet_id:"".into(), tx_hash: e }),
    };

    let provider = match Provider::<Http>::try_from(st.rpc_url.as_str()) {
        Ok(p)=>p, Err(e)=> return Json(CreateOutletResp{ ok:false, outlet_id:"".into(), tx_hash: format!("RPC_PROVIDER_ERR: {e}") }),
    };
    let wallet: LocalWallet = pk.parse().unwrap();
    let client = std::sync::Arc::new(SignerMiddleware::new(provider, wallet));

    let reg_addr: Address = ds.outletRegistry.parse().unwrap_or(Address::zero());
    if reg_addr == Address::zero() {
        return Json(CreateOutletResp{ ok:false, outlet_id:"".into(), tx_hash:"OUTLET_REGISTRY_NOT_DEPLOYED".into() });
    }
    let reg = OutletRegistry::new(reg_addr, client.clone());

    let outlet_id = match reg.outlet_id_from_domain(req.domain.clone()).call().await {
        Ok(id)=>id,
        Err(e)=> return Json(CreateOutletResp{ ok:false, outlet_id:"".into(), tx_hash: format!("ID_ERR: {e}") }),
    };

    let pending = match reg.create_outlet(req.name, req.domain).send().await {
        Ok(p)=>p,
        Err(e)=> return Json(CreateOutletResp{ ok:false, outlet_id: format!("{:#x}", outlet_id), tx_hash: format!("TX_ERR: {e}") }),
    };
    match pending.await {
        Ok(Some(r))=>{
            let ok = r.status.unwrap_or_default().as_u64()==1;
            Json(CreateOutletResp{ ok, outlet_id: format!("{:#x}", outlet_id), tx_hash: format!("{:#x}", r.transaction_hash) })
        }
        Ok(None)=> Json(CreateOutletResp{ ok:false, outlet_id: format!("{:#x}", outlet_id), tx_hash:"NO_RECEIPT".into() }),
        Err(e)=> Json(CreateOutletResp{ ok:false, outlet_id: format!("{:#x}", outlet_id), tx_hash: format!("RECEIPT_ERR: {e}") }),
    }
}

#[derive(Debug, Deserialize)]
struct DeployTokenReq {
    domain: String,
    token_name: String,
    token_symbol: String,
    minted_supply_wei: String,
    owner_private_key: Option<String>,
    test_transfer_to_self_wei: Option<String>,
}

#[derive(Serialize)]
struct DeployTokenResp {
    ok: bool,
    outlet_id: String,
    token_address: String,
    deploy_tx: String,
    test_tx: String,
}

async fn deploy_outlet_token(State(st): State<AppState>, Json(req): Json<DeployTokenReq>) -> Json<DeployTokenResp> {
    let ds = read_deploy_json(&st.state_dir);
    let pk = match load_owner_pk(&st, req.owner_private_key) {
        Ok(pk)=>pk,
        Err(e)=> return Json(DeployTokenResp{ ok:false, outlet_id:"".into(), token_address:"".into(), deploy_tx: e, test_tx:"".into() }),
    };

    let provider = match Provider::<Http>::try_from(st.rpc_url.as_str()) {
        Ok(p)=>p, Err(e)=> return Json(DeployTokenResp{ ok:false, outlet_id:"".into(), token_address:"".into(), deploy_tx: format!("RPC_PROVIDER_ERR: {e}"), test_tx:"".into() }),
    };
    let wallet: LocalWallet = pk.parse().unwrap();
    let from = wallet.address();
    let client = std::sync::Arc::new(SignerMiddleware::new(provider, wallet));

    let reg_addr: Address = ds.outletRegistry.parse().unwrap_or(Address::zero());
    let fac_addr: Address = ds.outletTokenFactory.parse().unwrap_or(Address::zero());
    if reg_addr==Address::zero() || fac_addr==Address::zero(){
        return Json(DeployTokenResp{ ok:false, outlet_id:"".into(), token_address:"".into(), deploy_tx:"OUTLET_REGISTRY_OR_FACTORY_NOT_DEPLOYED".into(), test_tx:"".into() });
    }
    let reg = OutletRegistry::new(reg_addr, client.clone());
    let outlet_id = match reg.outlet_id_from_domain(req.domain.clone()).call().await {
        Ok(id)=>id,
        Err(e)=> return Json(DeployTokenResp{ ok:false, outlet_id:"".into(), token_address:"".into(), deploy_tx: format!("ID_ERR: {e}"), test_tx:"".into() }),
    };

    let fac = OutletTokenFactory::new(fac_addr, client.clone());
    let minted: U256 = req.minted_supply_wei.parse::<U256>().unwrap_or_else(|_| U256::from(0u64));

    let pending = match fac.deploy_outlet_token(outlet_id, req.token_name, req.token_symbol, minted).send().await {
        Ok(p)=>p,
        Err(e)=> return Json(DeployTokenResp{ ok:false, outlet_id: format!("{:#x}", outlet_id), token_address:"".into(), deploy_tx: format!("DEPLOY_TX_ERR: {e}"), test_tx:"".into() }),
    };
    let deploy_tx_hash;
    match pending.await {
        Ok(Some(r))=>{
            let ok = r.status.unwrap_or_default().as_u64()==1;
            deploy_tx_hash = format!("{:#x}", r.transaction_hash);
            if !ok {
                return Json(DeployTokenResp{ ok:false, outlet_id: format!("{:#x}", outlet_id), token_address:"".into(), deploy_tx: deploy_tx_hash, test_tx:"".into() });
            }
        }
        Ok(None)=> return Json(DeployTokenResp{ ok:false, outlet_id: format!("{:#x}", outlet_id), token_address:"".into(), deploy_tx: "NO_RECEIPT".into(), test_tx:"".into() }),
        Err(e)=> return Json(DeployTokenResp{ ok:false, outlet_id: format!("{:#x}", outlet_id), token_address:"".into(), deploy_tx: format!("RECEIPT_ERR: {e}"), test_tx:"".into() }),
    }

    let tok_addr = match fac.outlet_token_of(outlet_id).call().await {
        Ok(a)=>a,
        Err(e)=> return Json(DeployTokenResp{ ok:false, outlet_id: format!("{:#x}", outlet_id), token_address:"".into(), deploy_tx: deploy_tx_hash, test_tx: format!("TOKEN_READ_ERR: {e}") }),
    };

    let mut test_tx = "SKIPPED".to_string();
    if let Some(x) = req.test_transfer_to_self_wei {
        let amt: U256 = x.parse::<U256>().unwrap_or_else(|_| U256::from(0u64));
        let erc = ERC20Mini::new(tok_addr, client.clone());
        let pending2 = match erc.transfer(from, amt).send().await {
            Ok(p)=>p,
            Err(e)=> return Json(DeployTokenResp{ ok:false, outlet_id: format!("{:#x}", outlet_id), token_address: format!("{:#x}", tok_addr), deploy_tx: deploy_tx_hash, test_tx: format!("TX_ERR:{e}") }),
        };
        match pending2.await {
            Ok(Some(r))=>{
                let ok = r.status.unwrap_or_default().as_u64()==1;
                test_tx = format!("{:#x}", r.transaction_hash);
                if !ok {
                    return Json(DeployTokenResp{ ok:false, outlet_id: format!("{:#x}", outlet_id), token_address: format!("{:#x}", tok_addr), deploy_tx: deploy_tx_hash, test_tx });
                }
            }
            Ok(None)=> return Json(DeployTokenResp{ ok:false, outlet_id: format!("{:#x}", outlet_id), token_address: format!("{:#x}", tok_addr), deploy_tx: deploy_tx_hash, test_tx: "NO_RECEIPT".into() }),
            Err(e)=> return Json(DeployTokenResp{ ok:false, outlet_id: format!("{:#x}", outlet_id), token_address: format!("{:#x}", tok_addr), deploy_tx: deploy_tx_hash, test_tx: format!("RECEIPT_ERR:{e}") }),
        }
    }

    Json(DeployTokenResp{
        ok: true,
        outlet_id: format!("{:#x}", outlet_id),
        token_address: format!("{:#x}", tok_addr),
        deploy_tx: deploy_tx_hash,
        test_tx,
    })
}

#[derive(Debug, Deserialize)]
struct ListReq { domain: String, token_address: String, tier: u8, owner_private_key: Option<String> }

#[derive(Serialize)]
struct ListResp { ok: bool, tx_hash: String }

async fn list_token(State(st): State<AppState>, Json(req): Json<ListReq>) -> Json<ListResp> {
    let ds = read_deploy_json(&st.state_dir);
    let pk = match load_owner_pk(&st, req.owner_private_key) {
        Ok(pk)=>pk,
        Err(e)=> return Json(ListResp{ ok:false, tx_hash: e }),
    };

    let provider = match Provider::<Http>::try_from(st.rpc_url.as_str()) {
        Ok(p)=>p, Err(e)=> return Json(ListResp{ ok:false, tx_hash: format!("RPC_PROVIDER_ERR: {e}") }),
    };
    let wallet: LocalWallet = pk.parse().unwrap();
    let client = std::sync::Arc::new(SignerMiddleware::new(provider, wallet));

    let reg_addr: Address = ds.outletRegistry.parse().unwrap_or(Address::zero());
    let list_addr: Address = ds.exchangeListingRegistry.parse().unwrap_or(Address::zero());
    if reg_addr==Address::zero() || list_addr==Address::zero() {
        return Json(ListResp{ ok:false, tx_hash:"REGISTRY_NOT_DEPLOYED".into() });
    }
    let reg = OutletRegistry::new(reg_addr, client.clone());
    let outlet_id = match reg.outlet_id_from_domain(req.domain.clone()).call().await {
        Ok(id)=>id,
        Err(e)=> return Json(ListResp{ ok:false, tx_hash: format!("ID_ERR:{e}") }),
    };
    let token: Address = req.token_address.parse().unwrap_or(Address::zero());
    if token==Address::zero(){ return Json(ListResp{ ok:false, tx_hash:"BAD_TOKEN".into() }); }
    let listing = ExchangeListingRegistry::new(list_addr, client.clone());

    let pending = match listing.list_token(token, outlet_id, req.tier).send().await {
        Ok(p)=>p,
        Err(e)=> return Json(ListResp{ ok:false, tx_hash: format!("TX_ERR:{e}") }),
    };
    match pending.await {
        Ok(Some(r))=>{
            let ok = r.status.unwrap_or_default().as_u64()==1;
            Json(ListResp{ ok, tx_hash: format!("{:#x}", r.transaction_hash) })
        }
        Ok(None)=> Json(ListResp{ ok:false, tx_hash:"NO_RECEIPT".into() }),
        Err(e)=> Json(ListResp{ ok:false, tx_hash: format!("RECEIPT_ERR:{e}") }),
    }
}

#[derive(Serialize)]
struct ArticleApprovalDefaults { vote_window_seconds: u64, community_min: u64, outlet_min: u64, council_min: u64 }

async fn approval_defaults() -> Json<ArticleApprovalDefaults> {
    Json(ArticleApprovalDefaults{ vote_window_seconds:259200, community_min:200, outlet_min:10, council_min:3 })
}

#[derive(Serialize)]
struct ArticleVoteResp {
    ok: bool,
    article_id: String,
    start_at: u64,
    end_at: u64,
    community: u32,
    outlet: u32,
    council: u32,
    flags: u32,
    finalized: bool,
    approved: bool,
    finalize_tx: String,
}

async fn article_votes(State(st): State<AppState>, axum::extract::Path(id): axum::extract::Path<u64>) -> Json<ArticleVoteResp> {
    let ds = read_deploy_json(&st.state_dir);
    let aa_addr: Address = ds.articleApprovals.parse().unwrap_or(Address::zero());
    if aa_addr == Address::zero() {
        return Json(ArticleVoteResp{
            ok:false, article_id: id.to_string(),
            start_at:0,end_at:0,community:0,outlet:0,council:0,flags:0,finalized:false,approved:false,
            finalize_tx:"ARTICLE_APPROVALS_NOT_DEPLOYED".into(),
        });
    }

    let provider = match Provider::<Http>::try_from(st.rpc_url.as_str()) {
        Ok(p)=>p, Err(e)=> return Json(ArticleVoteResp{
            ok:false, article_id: id.to_string(),
            start_at:0,end_at:0,community:0,outlet:0,council:0,flags:0,finalized:false,approved:false,
            finalize_tx: format!("RPC_PROVIDER_ERR:{e}"),
        }),
    };

    let aa_read = ArticleApprovals::new(aa_addr, std::sync::Arc::new(provider.clone()));

    let (start_at, end_at, community, outlet, council, flags, finalized, approved) =
        match aa_read.get_counts(U256::from(id)).call().await {
            Ok(v)=>v,
            Err(e)=> return Json(ArticleVoteResp{
                ok:false, article_id: id.to_string(),
                start_at:0,end_at:0,community:0,outlet:0,council:0,flags:0,finalized:false,approved:false,
                finalize_tx: format!("CALL_ERR:{e}"),
            })
        };

    let mut finalize_tx = "NONE".to_string();
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    if !finalized && end_at as u64 > 0 && now > end_at as u64 {
        // best-effort auto-finalize so explorers show a hard final state after the 72h window
        if let Ok(pk) = load_owner_pk(&st, None) {
            if let Ok(wallet) = pk.parse::<LocalWallet>() {
                let client = std::sync::Arc::new(SignerMiddleware::new(provider, wallet));
                let aa = ArticleApprovals::new(aa_addr, client);
                match aa.finalize(U256::from(id)).send().await {
                    Ok(pending) => {
                        match pending.await {
                            Ok(Some(r)) => {
                                let ok = r.status.unwrap_or_default().as_u64()==1;
                                finalize_tx = if ok { format!("{:#x}", r.transaction_hash) } else { format!("FAILED:{:#x}", r.transaction_hash) };
                            }
                            Ok(None) => { finalize_tx = "NO_RECEIPT".into(); }
                            Err(e) => { finalize_tx = format!("RECEIPT_ERR:{e}"); }
                        }
                    }
                    Err(e) => { finalize_tx = format!("FINALIZE_TX_ERR:{e}"); }
                }
            }
        }
    }

    // refresh counts after attempted finalize (best-effort)
    let (start_at2, end_at2, community2, outlet2, council2, flags2, finalized2, approved2) =
        match aa_read.get_counts(U256::from(id)).call().await {
            Ok(v)=>v,
            Err(_)=> (start_at, end_at, community, outlet, council, flags, finalized, approved)
        };

    Json(ArticleVoteResp{
        ok:true,
        article_id: id.to_string(),
        start_at: start_at2 as u64,
        end_at: end_at2 as u64,
        community: community2,
        outlet: outlet2,
        council: council2,
        flags: flags2,
        finalized: finalized2,
        approved: approved2,
        finalize_tx,
    })
}

#[tokio::main]
async 

async fn ops_metrics() -> axum::Json<serde_json::Value> {
    let up = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    axum::Json(serde_json::json!({
        "requests": REQUESTS.load(Ordering::Relaxed),
        "errors": ERRORS.load(Ordering::Relaxed),
        "epoch": up
    }))
}

async fn ops_health() -> axum::Json<serde_json::Value> {
    // Module-aware: read enabled modules from config/modules.json if present.
    let mut components = serde_json::Map::new();
    let enabled = load_enabled_modules();
    for (k,v) in enabled.iter() {
        if !*v { continue; }
        // Minimal health surfaces by convention
        let status = match k.as_str() {
            "indexer" => probe_http("http://press-indexer:8096/health").await,
            "blockscout" => probe_http("http://blockscout:4000").await,
            "edge_proxy" => probe_http("http://press-edge-proxy:8085").await,
            "bots" => probe_http("http://bots-dashboard:8088/health").await,
            _ => Ok(())
        }.is_ok();
        components.insert(k.clone(), serde_json::json!({"ok": status}));
    }
    // Core: gateway is ok if we are here; rpc is checked
    let rpc_ok = probe_rpc("http://press-rpc:8545").await.is_ok();
    components.insert("rpc".into(), serde_json::json!({"ok": rpc_ok}));
    axum::Json(serde_json::json!({
        "ok": rpc_ok,
        "components": components
    }))
}

fn load_enabled_modules() -> std::collections::HashMap<String,bool> {
    let mut out = std::collections::HashMap::new();
    // defaults
    out.insert("indexer".into(), true);
    out.insert("blockscout".into(), true);
    out.insert("edge_proxy".into(), true);
    out.insert("bots".into(), true);

    if let Ok(s) = fs::read_to_string("config/modules.json") {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            if let Some(arr) = v.get("modules").and_then(|x| x.as_array()) {
                out.clear();
                for m in arr {
                    let id = m.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                    let en = m.get("enabled").and_then(|x| x.as_bool()).unwrap_or(true);
                    if !id.is_empty() { out.insert(id,en); }
                }
            }
        }
    }
    out
}

async fn probe_http(url: &str) -> Result<(), ()> {
    let client = reqwest::Client::new();
    let res = client.get(url).timeout(std::time::Duration::from_secs(2)).send().await.map_err(|_| ())?;
    if res.status().is_success() { Ok(()) } else { Err(()) }
}

async fn probe_rpc(url: &str) -> Result<(), ()> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]});
    let res = client.post(url).json(&body).timeout(std::time::Duration::from_secs(2)).send().await.map_err(|_| ())?;
    if res.status().is_success() { Ok(()) } else { Err(()) }
}
fn main() {
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    let st = AppState{ state_dir: state_dir(), rpc_url: rpc_url_from_env() };
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    let app = Router::new().layer(rate_limit::layer()).layer(track_requests())
        .route("/health", get(health))
        .route("/api/outlet/info", get(wizard_info))
        .route("/api/outlets/create", post(create_outlet))
        .route("/api/outlets/token/deploy", post(deploy_outlet_token))
        .route("/api/exchange/list", post(list_token))
        .route("/api/articles/approval_defaults", get(approval_defaults))
        .route("/api/articles/votes/:id", get(article_votes))
.route("/api/config/modules", get(config_modules))
        .route("/api/config/modules/write", post(write_config_modules))
.route("/api/config/listing-tiers", get(config_listing_tiers))
.route("/api/config/brand", get(config_brand))
        .route("/api/config/release-presets", get(config_release_presets))
        .route("/api/health", get(health))
        .route("/api/deploy/snapshot", get(deploy_snapshot))
        .route("/api/deploy/snapshot/write", post(write_deploy_snapshot))
        .route("/api/deploy/auto-fix", post(run_auto_fix))

        .layer(cors)
        .with_state(st);

    let addr: SocketAddr = "0.0.0.0:8090".parse().unwrap();
    info!("press_gateway_api listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app).await.unwrap();
}
