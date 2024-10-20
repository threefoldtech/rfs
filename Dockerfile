FROM rust:slim as builder

WORKDIR /src

COPY fl-server /src/fl-server
COPY rfs /src/rfs
COPY docker2fl /src/docker2fl
COPY Cargo.toml .
COPY Cargo.lock .
COPY config.toml .

RUN apt-get update && apt-get install curl build-essential libssl-dev musl-tools -y
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --bin fl-server --target=x86_64-unknown-linux-musl

FROM alpine:3.19

WORKDIR /app

COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/fl-server .
COPY --from=builder /src/config.toml .

ENTRYPOINT [ "./fl-server", "--config-path", "config.toml"]
