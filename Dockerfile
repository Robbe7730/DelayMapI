FROM rust:1.54.0

WORKDIR /app
COPY . .

RUN apt update
RUN apt install -y protobuf-compiler
RUN cargo install --path .

CMD ["delaymapi"]
