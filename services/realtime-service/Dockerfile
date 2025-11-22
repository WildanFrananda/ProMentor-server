FROM rust:1.85 AS builder

RUN apt-get update && apt-get install -y \
    musl-tools \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /usr/src/realtime-service

COPY . .

RUN cargo build --release --target=x86_64-unknown-linux-musl --offline


FROM alpine:latest

RUN apk add --no-cache ca-certificates

WORKDIR /root/

COPY --from=builder /usr/src/realtime-service/target/x86_64-unknown-linux-musl/release/realtime-service .

EXPOSE 8080

CMD ["./realtime-service"]