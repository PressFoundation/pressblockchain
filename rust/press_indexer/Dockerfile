FROM rust:1.78-slim as build
WORKDIR /src
COPY . /src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /src/target/release/press_indexer /app/press_indexer
EXPOSE 8096
ENV RUST_LOG=info
CMD ["/app/press_indexer"]
