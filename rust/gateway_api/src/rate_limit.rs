use std::time::Duration;
use tower::limit::RateLimitLayer;

pub fn layer() -> RateLimitLayer {
    // 50 req/sec burst-safe; adjust via env later
    RateLimitLayer::new(50, Duration::from_secs(1))
}
