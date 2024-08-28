FROM rust:slim

RUN apt-get update && apt-get install curl build-essential libssl-dev musl-tools -y

WORKDIR /myapp

COPY . .

RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target=x86_64-unknown-linux-musl

CMD ["/myapp/target/x86_64-unknown-linux-musl/release/fl-server", "--config-path", "config.toml"]
EXPOSE 3000
