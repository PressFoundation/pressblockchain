use axum::{routing::get, Json, Router};
use serde::Serialize;
use std::net::SocketAddr;

#[derive(Serialize)]
struct Probe { name: String, ok: bool, url: String }

#[derive(Serialize)]
struct Status { ok: bool, probes: Vec<Probe> }

async fn status() -> Json<Status> {
  let checks = vec![
    ("rpc","http://press-rpc:8545"),
    ("articles","http://press-articles:8808/health"),
    ("oracle","http://press-oracle:8811/health"),
    ("treasury","http://press-treasury:8807/health"),
  ];
  let client = reqwest::Client::new();
  let mut probes = Vec::new();
  let mut all_ok = true;
  for (name,url) in checks {
    let ok = client.get(url).send().await.map(|r| r.status().is_success()).unwrap_or(false);
    if !ok { all_ok = false; }
    probes.push(Probe{name:name.into(), ok, url:url.into()});
  }
  Json(Status{ ok: all_ok, probes })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let addr: SocketAddr = "0.0.0.0:8812".parse().unwrap();
  let app = Router::new()
    .route("/health", get(|| async {"ok"}))
    .route("/v1/status", get(status));
  axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
  Ok(())
}
