\
use axum::{routing::{get, post}, Json, Router, extract::{Query, State}};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr, sync::{Arc, Mutex}};
use uuid::Uuid;
use ethers::types::{Address, Signature, H256};
use ethers::utils::keccak256;
use jsonwebtoken::{encode, EncodingKey, Header};
use time::{Duration, OffsetDateTime};
use tower_http::cors::{CorsLayer, Any};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
struct AppState {
    nonces: Arc<Mutex<HashMap<String, String>>>,
    jwt_secret: String,
    chain_id: u64,
    domain: String,
}

#[derive(Deserialize)]
struct NonceQuery {
    address: String,
}

#[derive(Serialize)]
struct NonceResp {
    address: String,
    nonce: String,
    message: String,
}

#[derive(Deserialize)]
struct VerifyReq {
    address: String,
    signature: String,
}

#[derive(Serialize)]
struct MeResp { address: String, chain_id: u64, domain: String, roles: Vec<String> }

struct VerifyResp {
    token: String,
    expires_at: i64,
}

#[derive(Serialize, Deserialize)]
#[derive(Serialize, Deserialize)]
struct RolesFile { ROLE_OUTLET_BOND: String, ROLE_JOURNALIST_BOND: String, ROLE_COUNCIL_BOND: String }

struct Claims {
    sub: String,
    exp: usize,
    iat: usize,
    chain_id: u64,
    domain: String,
}

fn build_message(domain: &str, chain_id: u64, address: &str, nonce: &str) -> String {
    // Not full SIWE; intentionally simple and deterministic.
    // Wallet signs a human-readable statement bound to domain + chain.
    format!(
        "Press Wallet Login\nDomain: {}\nChainId: {}\nAddress: {}\nNonce: {}\nStatement: Sign to authenticate with Press Blockchain apps.\n",
        domain, chain_id, address, nonce
    )
}

fn hash_message(message: &str) -> H256 {
    // Ethereum personal_sign prefix
    let prefix = format!("\x19Ethereum Signed Message:\n{}", message.len());
    H256::from_slice(&keccak256([prefix.as_bytes(), message.as_bytes()].concat()))
}

\

use axum::http::HeaderMap;
use jsonwebtoken::{decode, DecodingKey, Validation};
use ethers::providers::{Provider, Http};
use ethers::contract::abigen;

abigen!(CouncilRegistry, r#"[function isCouncil(address) view returns (bool)]"#);
abigen!(BondVault, r#"[function bonded(address,bytes32) view returns (uint256)]"#);

#[derive(Deserialize)]
struct MeQuery { address: Option<String> }

async fn me(State(st): State<AppState>, headers: HeaderMap, Query(q): Query<MeQuery>) -> Result<Json<MeResp>, (axum::http::StatusCode, String)> {
    let auth = headers.get("authorization").and_then(|v| v.to_str().ok()).unwrap_or("");
    if !auth.starts_with("Bearer ") {
        return Err((axum::http::StatusCode::UNAUTHORIZED, "NO_BEARER".into()));
    }
    let token = auth.trim_start_matches("Bearer ").trim();

    let data = decode::<Claims>(token, &DecodingKey::from_secret(st.jwt_secret.as_bytes()), &Validation::default())
        .map_err(|_| (axum::http::StatusCode::UNAUTHORIZED, "BAD_TOKEN".into()))?;

    let address = data.claims.sub.clone();
    // Optional on-chain role claims (best-effort). If RPC not set, returns empty.
    let mut roles: Vec<String> = vec![];

    if let Ok(rpc) = std::env::var("PRESS_RPC_HTTP") {
        if let Ok(provider) = Provider::<Http>::try_from(rpc) {
            // council role
            if let Ok(cr_addr) = std::env::var("COUNCIL_REGISTRY_ADDR") {
                if let Ok(cr) = cr_addr.parse() {
                    let c = CouncilRegistry::new(cr, provider.clone());
                    if let Ok(is_c) = c.is_council(address.parse().unwrap()).call().await {
                        if is_c { roles.push("council".into()); }
                    }
                }
            }
                    if let Ok(j_role) = std::env::var("ROLE_JOURNALIST_BOND_HEX") {
                        if let Ok(bytes) = hex::decode(j_role.trim_start_matches("0x")) {
                            if bytes.len()==32 {
                                let mut arr=[0u8;32]; arr.copy_from_slice(&bytes);
                                if let Ok(v) = b.bonded(address.parse().unwrap(), arr.into()).call().await {
                                    if v > 0u64.into() { roles.push("journalist".into()); }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(Json(MeResp{ address, chain_id: st.chain_id, domain: st.domain, roles }))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let jwt_secret = std::env::var("AUTH_JWT_SECRET").unwrap_or_else(|_| "dev-insecure-change-me".to_string());
    let chain_id = std::env::var("PRESS_CHAIN_ID").ok().and_then(|v| v.parse().ok()).unwrap_or(777777);
    let domain = std::env::var("PRESS_AUTH_DOMAIN").unwrap_or_else(|_| "pressblockchain.io".to_string());

    let state = AppState {
        nonces: Arc::new(Mutex::new(HashMap::new())),
        jwt_secret,
        chain_id,
        domain,
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/nonce", get(get_nonce))
        .route("/verify", post(verify))
        .route("/me", get(me))
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
        .with_state(state);

    let addr: SocketAddr = "0.0.0.0:8788".parse().unwrap();
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app).await.unwrap();
}

async fn get_nonce(State(st): State<AppState>, Query(q): Query<NonceQuery>) -> Json<NonceResp> {
    let nonce = Uuid::new_v4().to_string();
    let address = q.address.to_lowercase();
    st.nonces.lock().unwrap().insert(address.clone(), nonce.clone());
    let message = build_message(&st.domain, st.chain_id, &address, &nonce);
    Json(NonceResp { address, nonce, message })
}

async fn verify(State(st): State<AppState>, Json(req): Json<VerifyReq>) -> Result<Json<VerifyResp>, (axum::http::StatusCode, String)> {
    let address_str = req.address.to_lowercase();
    let nonce = st.nonces.lock().unwrap().remove(&address_str).ok_or((axum::http::StatusCode::BAD_REQUEST, "NO_NONCE".to_string()))?;

    let msg = build_message(&st.domain, st.chain_id, &address_str, &nonce);
    let digest = hash_message(&msg);

    let sig: Signature = req.signature.parse().map_err(|_| (axum::http::StatusCode::BAD_REQUEST, "BAD_SIG".to_string()))?;
    let recovered = sig.recover(digest).map_err(|_| (axum::http::StatusCode::UNAUTHORIZED, "RECOVER_FAIL".to_string()))?;

    let expected: Address = address_str.parse().map_err(|_| (axum::http::StatusCode::BAD_REQUEST, "BAD_ADDR".to_string()))?;
    if recovered != expected {
        return Err((axum::http::StatusCode::UNAUTHORIZED, "ADDR_MISMATCH".to_string()));
    }

    let now = OffsetDateTime::now_utc();
    let exp = now + Duration::hours(12);

    let claims = Claims {
        sub: address_str.clone(),
        iat: now.unix_timestamp() as usize,
        exp: exp.unix_timestamp() as usize,
        chain_id: st.chain_id,
        domain: st.domain.clone(),
    };

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(st.jwt_secret.as_bytes()))
        .map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "JWT_FAIL".to_string()))?;

    Ok(Json(VerifyResp { token, expires_at: exp.unix_timestamp() }))
}
