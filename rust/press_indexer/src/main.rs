use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::RwLock;

#[derive(Default, Clone)]
struct State {
    chain_id: String,
    latest_block: u64,
}

#[derive(Serialize)]
struct Stats {
    chain_id: String,
    latest_block: u64,
}

async fn health() -> &'static str { "ok" }

async fn stats(st: axum::extract::State<Arc<RwLock<State>>>) -> Json<Stats> {
    let s = st.read().await;
    Json(Stats { chain_id: s.chain_id.clone(), latest_block: s.latest_block })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc = std::env::var("PRESS_RPC_HTTP").unwrap_or_else(|_| "http://press-rpc:8545".to_string());
    let addr: SocketAddr = "0.0.0.0:8799".parse().unwrap();
    let st = Arc::new(RwLock::new(State::default()));

    // background poller (production-safe: no panic loops)
    {
        let st = st.clone();
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            loop {
                if let Ok(cid) = client.post(&rpc).json(&serde_json::json!({
                    "jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]
                })).send().await.and_then(|r| r.json::<serde_json::Value>())
                    .await
                    .ok()
                    .and_then(|v| v.get("result").and_then(|r| r.as_str()).map(|s| s.to_string()))
                {
                    st.write().await.chain_id = cid;
                }

                if let Ok(bn) = client.post(&rpc).json(&serde_json::json!({
                    "jsonrpc":"2.0","id":2,"method":"eth_blockNumber","params":[]
                })).send().await.and_then(|r| r.json::<serde_json::Value>()).await
                {
                    if let Some(hex) = bn.get("result").and_then(|r| r.as_str()) {
                        if let Ok(v) = u64::from_str_radix(hex.trim_start_matches("0x"), 16) {
                            st.write().await.latest_block = v;
                        }
                    }
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
    }

    let app = Router::new()
        .route("/health", get(health))
        .route("/stats", get(stats))
        .with_state(st);

    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
