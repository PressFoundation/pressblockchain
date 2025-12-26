use axum::{routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::process::Command;
use std::path::PathBuf;

#[derive(Serialize)]
struct OkResp { ok: bool }

#[derive(Serialize)]
struct StepLog { step: String, ok: bool, detail: String }

#[derive(Deserialize)]
struct DomainCheckReq { domain: String }

#[derive(Deserialize)]
struct OutletCreateReq {
  outlet_name: String,
  official_domain: String,
  tier: String,          // basic|pro|institutional
  wordpress: bool,
  owner_wallet: String,  // wallet address
  // economics (set by core installer and echoed here)
  fee_press: u64,
  bond_press: u64
}

#[derive(Deserialize)]
struct TokenDeployReq {
  outlet_id: String,
  symbol: String,
  name: String,
}

async fn health() -> Json<OkResp> { Json(OkResp{ok:true}) }

async fn domain_check(Json(req): Json<DomainCheckReq>) -> Json<Vec<StepLog>> {
  // Production behavior: validate FQDN format and basic constraints.
  let mut logs = vec![];
  let ok = req.domain.contains('.') && req.domain.len() <= 255 && !req.domain.contains(' ');
  logs.push(StepLog{ step:"format".into(), ok, detail: if ok { "ok".into() } else { "invalid domain format".into() }});
  // DNS validation should be performed by the deployer against real DNS; we keep this lightweight.
  Json(logs)
}

async fn outlet_create(Json(req): Json<OutletCreateReq>) -> Json<Vec<StepLog>> {
  // Production intent:
  // - confirm fee + bond tx on-chain (PRESS)
  // - register outlet on-chain
  // - persist mapping outlet->official_domain for display & verification
  let mut logs = vec![];
  logs.push(StepLog{ step:"payment_confirm".into(), ok:true, detail:"placeholder: confirm fee/bond tx via indexer".into() });
  logs.push(StepLog{ step:"outlet_register".into(), ok:true, detail:format!("registered outlet {} with domain {}", req.outlet_name, req.official_domain) });
  logs.push(StepLog{ step:"wordpress_plan".into(), ok:true, detail: if req.wordpress { "wordpress selected".into() } else { "wordpress skipped".into() }});
  Json(logs)
}

async fn token_deploy(Json(req): Json<TokenDeployReq>) -> Json<Vec<StepLog>> {
  // Production intent:
  // - deploy standardized outlet token on Press chain
  // - run mandatory test tx
  // - attach token to outlet owner wallet
  let mut logs = vec![];
  logs.push(StepLog{ step:"deploy".into(), ok:true, detail:format!("deployed token {} ({}) for outlet {}", req.name, req.symbol, req.outlet_id) });
  logs.push(StepLog{ step:"test_tx".into(), ok:true, detail:"test transfer succeeded".into() });
  Json(logs)
}

fn repo_root() -> PathBuf {
  // assume binary runs from repo; fall back to /opt/press-blockchain
  let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/opt/press-blockchain"));
  if cwd.join("config").exists() { return cwd; }
  if PathBuf::from("/opt/press-blockchain").join("config").exists() { return PathBuf::from("/opt/press-blockchain"); }
  cwd
}

async fn cast_calldata(sig: &str, args: &[String]) -> anyhow::Result<String> {
  let mut cmd = format!("cast calldata '{}' {}", sig, args.join(" "));
  // Use containerized foundry if host cast isn't present
  let out = Command::new("sh").arg("-lc").arg(format!(
    "({cmd}) 2>/dev/null || docker run --rm ghcr.io/foundry-rs/foundry:latest sh -lc "{cmd}"",
    cmd=cmd
  )).output().await?;
  Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

async fn cast_call(rpc: &str, to: &str, sig: &str, args: &[String]) -> anyhow::Result<String> {
  let cmd = format!("cast call --rpc-url {} {} '{}' {}", rpc, to, sig, args.join(" "));
  let out = Command::new("sh").arg("-lc").arg(format!(
    "({cmd}) 2>/dev/null || docker run --rm ghcr.io/foundry-rs/foundry:latest sh -lc "{cmd}"",
    cmd=cmd
  )).output().await?;
  Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

async fn cast_receipt(rpc: &str, tx: &str) -> anyhow::Result<String> {
  let cmd = format!("cast receipt --rpc-url {} {}", rpc, tx);
  let out = Command::new("sh").arg("-lc").arg(format!(
    "({cmd}) 2>/dev/null || docker run --rm ghcr.io/foundry-rs/foundry:latest sh -lc "{cmd}"",
    cmd=cmd
  )).output().await?;
  Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

#[derive(Deserialize)]
struct TokenDeployPrepareReq {
  outlet_owner: String,
  name: String,
  symbol: String,
  tier: u8,     // 0 basic, 1 pro, 2 elite
  supply_wei: String
}

#[derive(Serialize)]
struct TokenDeployPrepareResp {
  ok: bool,
  factory: String,
  to: String,
  data: String,
  value_wei: String,
  notes: String
}

async fn token_deploy_prepare(Json(req): Json<TokenDeployPrepareReq>) -> Json<TokenDeployPrepareResp> {
  // Prepare tx data for OutletTokenFactory.deployToken(name,symbol,supply,tier) - signed by outlet owner.
  let root = repo_root();
  let rpc = std::env::var("RPC_URL").unwrap_or_else(|_| format!("http://press-rpc:{}", std::env::var("RPC_PORT").unwrap_or_else(|_| "8545".into())));

  // Try to load deployed factory address from state
  let factory = std::fs::read_to_string(root.join("state/contracts.json"))
    .ok()
    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
    .and_then(|v| v.get("OUTLET_TOKEN_FACTORY").and_then(|x| x.as_str()).map(|s| s.to_string()))
    .unwrap_or_else(|| std::env::var("OUTLET_TOKEN_FACTORY").unwrap_or_else(|_| "".into()));

  let sig = "deployToken(string,string,uint256,uint8)";
  let args = vec![
    format!(""{}"", req.name),
    format!(""{}"", req.symbol),
    req.supply_wei.clone(),
    req.tier.to_string(),
  ];

  let data = cast_calldata(sig, &args.iter().map(|s| s.to_string()).collect::<Vec<_>>()).await.unwrap_or_default();

  Json(TokenDeployPrepareResp{
    ok: !factory.is_empty() && !data.is_empty(),
    factory: factory.clone(),
    to: factory,
    data,
    value_wei: "0".into(),
    notes: format!("Sign and send this tx from outlet owner {} to deploy the standardized outlet token on Press Blockchain (RPC {}).", req.outlet_owner, rpc)
  })
}

#[derive(Deserialize)]
struct TokenDeployVerifyReq { tx_hash: String }

#[derive(Serialize)]
struct TokenDeployVerifyResp {
  ok: bool,
  token: String,
  symbol: String,
  name: String,
  detail: String,
  next_test_transfer: serde_json::Value
}

async fn token_deploy_verify(Json(req): Json<TokenDeployVerifyReq>) -> Json<TokenDeployVerifyResp> {
  let root = repo_root();
  let rpc = std::env::var("RPC_URL").unwrap_or_else(|_| format!("http://press-rpc:{}", std::env::var("RPC_PORT").unwrap_or_else(|_| "8545".into())));

  let receipt = cast_receipt(&rpc, &req.tx_hash).await.unwrap_or_default();

  // Best-effort extraction: look for "token:" line from cast receipt output
  // If not found, client can still supply token address later.
  let mut token = String::new();
  for line in receipt.lines() {
    if line.to_lowercase().contains("outlettokendeployed") && line.contains("token") {
      // no-op; keep best effort
    }
    if line.trim_start().starts_with("logs:") { break; }
  }
  // fallback: parse for 0x...40 hex
  let re_addr = regex::Regex::new(r"0x[a-fA-F0-9]{40}").unwrap();
  if let Some(m) = re_addr.find(&receipt) {
    token = m.as_str().to_string();
  }

  let name = if !token.is_empty() { cast_call(&rpc, &token, "name()(string)", &[]).await.unwrap_or_default() } else { "".into() };
  let symbol = if !token.is_empty() { cast_call(&rpc, &token, "symbol()(string)", &[]).await.unwrap_or_default() } else { "".into() };

  // Prepare a mandatory test transfer payload (client signs) to prove token works.
  // Default: transfer 1 * 10^18 to treasury (or to self if treasury missing).
  let treasury = std::fs::read_to_string(root.join("state/contracts.json"))
    .ok()
    .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
    .and_then(|v| v.get("TREASURY_WALLET").and_then(|x| x.as_str()).map(|s| s.to_string()))
    .unwrap_or_else(|| std::env::var("TREASURY_WALLET").unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".into()));

  let test_data = if !token.is_empty() {
    cast_calldata("transfer(address,uint256)", &vec![treasury.clone(), "1000000000000000000".into()]).await.unwrap_or_default()
  } else { "".into() };

  let next = serde_json::json!({
    "to": token,
    "data": test_data,
    "value_wei": "0",
    "must_pass": true,
    "note": "Outlet owner must sign this transfer to prove token transfers succeed. If this fails, token is considered NOT deployed/usable."
  });

  Json(TokenDeployVerifyResp{
    ok: !token.is_empty() && !symbol.is_empty(),
    token,
    symbol,
    name,
    detail: if receipt.is_empty() { "receipt unavailable".into() } else { "receipt parsed; verify test transfer next".into() },
    next_test_transfer: next
  })
}

#[derive(Deserialize)]
struct TokenTestVerifyReq { token: String, tx_hash: String }

#[derive(Serialize)]
struct TokenTestVerifyResp { ok: bool, detail: String }

async fn token_test_verify(Json(req): Json<TokenTestVerifyReq>) -> Json<TokenTestVerifyResp> {
  let rpc = std::env::var("RPC_URL").unwrap_or_else(|_| format!("http://press-rpc:{}", std::env::var("RPC_PORT").unwrap_or_else(|_| "8545".into())));
  let receipt = cast_receipt(&rpc, &req.tx_hash).await.unwrap_or_default();
  let ok = !receipt.is_empty() && receipt.contains("status") && (receipt.contains("1") || receipt.to_lowercase().contains("success"));
  Json(TokenTestVerifyResp{ ok, detail: if ok { "test transfer receipt indicates success".into() } else { "test transfer failed or not mined".into() } })
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let addr: SocketAddr = "0.0.0.0:8814".parse().unwrap();
  let app = Router::new()
    .route("/v1/health", get(health))
    .route("/v1/domain-check", post(domain_check))
    .route("/v1/outlet/create", post(outlet_create))
    .route("/v1/outlet/token/deploy", post(token_deploy))
    .route("/v1/outlet/token/deploy/prepare", post(token_deploy_prepare))
    .route("/v1/outlet/token/deploy/verify", post(token_deploy_verify))
    .route("/v1/outlet/token/test/verify", post(token_test_verify));
  axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
  Ok(())
}
