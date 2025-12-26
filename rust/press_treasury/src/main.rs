use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;

#[derive(Serialize)]
struct Status {
    ok: bool,
    treasury_balance_press: u64,
    next_burn_epoch: String
}

async fn status() -> Json<Status> {
    // Production: query on-chain treasury balance + schedule
    Json(Status{ ok:true, treasury_balance_press: 0, next_burn_epoch: "yearly".into() })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = "0.0.0.0:8807".parse().unwrap();
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/v1/status", get(status));
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
