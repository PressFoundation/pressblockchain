use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;

#[derive(Serialize)]
struct Status { ok: bool }

async fn status() -> Json<Status> { Json(Status{ok:true}) }

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr: SocketAddr = "0.0.0.0:8810".parse().unwrap();
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/v1/status", get(status));
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}
