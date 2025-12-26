FROM rust:1.82-slim as builder
WORKDIR /app
COPY rust ./rust
COPY Cargo.toml ./Cargo.toml
RUN apt-get update && apt-get install -y pkg-config ca-certificates && rm -rf /var/lib/apt/lists/*
RUN cargo build -p press_status_service --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
ENV BIND=0.0.0.0:8791
ENV PRESS_STATUS_STATE=/state/status_config.json
COPY --from=builder /app/target/release/press_status_service /app/press_status_service
EXPOSE 8791
CMD ["/app/press_status_service"]
