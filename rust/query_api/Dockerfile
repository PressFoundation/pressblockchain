FROM rust:1.82-slim as builder
WORKDIR /app
COPY rust ./rust
COPY Cargo.toml ./Cargo.toml
RUN apt-get update && apt-get install -y pkg-config ca-certificates && rm -rf /var/lib/apt/lists/*
RUN cargo build -p press_query_api --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/press_query_api /app/press_query_api
ENV PORT=8787
EXPOSE 8787
CMD ["/app/press_query_api"]
