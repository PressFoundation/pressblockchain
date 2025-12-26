use axum::{routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::net::SocketAddr;

#[derive(Serialize)]
struct Health { ok: bool }

#[derive(Deserialize)]
struct VerifyReq {
  title: String,
  body: String,
  canonical_url: Option<String>
}

#[derive(Serialize)]
struct VerifyResp {
  ok: bool,
  content_hash: String,
  flags: Vec<String>,
  tags: Vec<String>,
  confidence: f64
}

fn hash_content(title:&str, body:&str) -> String {
  let mut h = Sha256::new();
  h.update(title.as_bytes());
  h.update(b"\n");
  h.update(body.as_bytes());
  hex::encode(h.finalize())
}

fn simple_tags(title:&str, body:&str) -> Vec<String> {
  let s = format!("{} {}", title, body).to_lowercase();
  let mut tags = Vec::new();
  for (k,t) in [
    ("bitcoin","bitcoin"),("ethereum","ethereum"),("solana","solana"),
    ("election","politics"),("sec ","regulation"),("lawsuit","legal"),
    ("exchange","exchange"),("hack","security"),("breach","security"),
    ("ai ","ai"),("copyright","copyright"),("nft","nft"),
  ] {
    if s.contains(k) { tags.push(t.to_string()); }
  }
  tags.sort(); tags.dedup(); tags
}

fn flags(body:&str) -> Vec<String> {
  let b = body.to_lowercase();
  let mut f = Vec::new();
  // Basic safety gate for MVP: profanity/explicit content placeholders; production should use a classifier.
  for bad in ["porn", "sexual", "gore", "kill", "terrorism", "illegal"] {
    if b.contains(bad) { f.push(format!("content_flag:{}", bad)); }
  }
  // Very light "copyright risk" heuristic
  if body.len() > 8000 { f.push("copyright_risk:long_form".into()); }
  f
}

async fn health() -> Json<Health> { Json(Health{ok:true}) }

async fn config() -> Json<serde_json::Value> {
  Json(serde_json::from_str(include_str!("../../../config/oracle.json")).unwrap())
}

async fn verify(Json(req): Json<VerifyReq>) -> Json<VerifyResp> {
  let h = hash_content(&req.title, &req.body);
  let tags = simple_tags(&req.title, &req.body);
  let mut fl = flags(&req.body);
  if let Some(url) = req.canonical_url.as_ref() {
    if url.contains("arweave") { fl.push("import_flag:arweave".into()); }
  }
  let confidence = if fl.is_empty() { 0.93 } else { 0.55 };
  Json(VerifyResp{ ok:true, content_hash:h, flags:fl, tags, confidence })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let addr: SocketAddr = "0.0.0.0:8811".parse().unwrap();
  let app = Router::new()
    .route("/health", get(|| async {"ok"}))
    .route("/v1/health", get(health))
    .route("/v1/config", get(config))
    .route("/v1/verify/article", post(verify));
  axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
  Ok(())
}
