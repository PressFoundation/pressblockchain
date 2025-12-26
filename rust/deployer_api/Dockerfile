FROM rust:1.82-slim as builder
WORKDIR /app
COPY rust ./rust
COPY Cargo.toml ./Cargo.toml
RUN apt-get update && apt-get install -y pkg-config ca-certificates && rm -rf /var/lib/apt/lists/*
RUN cargo build -p press_deployer_api --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl wget iproute2 docker.io && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/press_deployer_api /app/press_deployer_api
ENV PORT=8085
ENV STATE_DIR=/state
EXPOSE 8085
CMD ["/app/press_deployer_api"]
