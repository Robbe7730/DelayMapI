FROM rust:latest as builder

WORKDIR /app

# Install protobuf-compiler
RUN apt update
RUN apt install -y protobuf-compiler

# Install rust core
RUN rustup install nightly

# Build & install dependencies
COPY Cargo.toml Cargo.lock dummy.rs rust-toolchain /app/
RUN sed -i 's#src/main.rs#dummy.rs#' Cargo.toml
RUN cargo build --release
RUN sed -i 's#dummy.rs#src/main.rs#' Cargo.toml

# Build delaymapi
COPY src/ protos/ build.rs /app/src/
RUN cargo install --path .

FROM debian:bullseye-slim as runner

COPY --from=builder /usr/local/cargo/bin/delaymapi /usr/local/bin/delaymapi

CMD ["delaymapi"]
