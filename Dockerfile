FROM rust:latest

WORKDIR /app
COPY Cargo.toml Cargo.lock /app/

RUN apt update
RUN apt install -y protobuf-compiler

COPY src/ /app/src/
RUN cargo install --path .

CMD ["delaymapi"]
