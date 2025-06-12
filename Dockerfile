FROM rust:slim as builder

WORKDIR /src

COPY rfs /src/rfs
COPY Cargo.toml .
COPY Cargo.lock .
COPY config.toml .

RUN apt-get update && apt-get install curl build-essential libssl-dev musl-tools -y
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target=x86_64-unknown-linux-musl

FROM alpine:3.19

WORKDIR /app

COPY --from=builder /src/target/x86_64-unknown-linux-musl/release/rfs .
COPY --from=builder /src/config.toml .

ENTRYPOINT [ "./rfs", "server", "--config-path", "config.toml"]
