FROM rust:1.78-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
RUN mkdir -p src && echo "fn main(){}" > src/main.rs
RUN cargo build --release || true
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/press_gateway_api /usr/local/bin/press_gateway_api
# RR23: ship devsuite static sources
COPY ../../apps/devsuite /app/devsuite
COPY ../../apps/outlet_wizard /app/outlet_wizard
COPY ../../apps/exchange_portal /app/exchange_portal
COPY ../../apps/proposal_center /app/proposal_center
EXPOSE 8085
ENV RUST_LOG=info
CMD ["/usr/local/bin/press_gateway_api"]
