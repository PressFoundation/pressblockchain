FROM rust:1.78-slim as builder
ARG BIN
WORKDIR /app
COPY . .
RUN cargo build -p ${BIN} --release

FROM debian:bookworm-slim
ARG BIN
WORKDIR /app
COPY --from=builder /app/target/release/${BIN} /usr/local/bin/${BIN}
EXPOSE 8799 8801 8803
ENTRYPOINT ["/usr/local/bin/${BIN}"]
