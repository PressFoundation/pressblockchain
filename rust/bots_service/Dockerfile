FROM rust:1.82-slim as builder
WORKDIR /app
COPY rust ./rust
COPY Cargo.toml ./Cargo.toml
RUN apt-get update && apt-get install -y pkg-config ca-certificates && rm -rf /var/lib/apt/lists/*
RUN cargo build -p press_bots_service --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
ENV BIND=0.0.0.0:8790
ENV PRESS_BOTS_STATE=/state/bots_config.json
COPY --from=builder /app/target/release/press_bots_service /app/press_bots_service
EXPOSE 8790
HEALTHCHECK --interval=15s --timeout=3s --retries=6 CMD curl -fsS http://127.0.0.1:8790/health || exit 1
CMD ["/app/press_bots_service"]
