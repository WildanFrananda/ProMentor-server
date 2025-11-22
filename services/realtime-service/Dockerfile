FROM rust:1.85 AS builder

RUN apt-get update && apt-get install -y \
    musl-tools \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl

RUN mkdir -p /root/.cargo && \
    echo '[net]' > /root/.cargo/config.toml && \
    echo 'retry = 10' >> /root/.cargo/config.toml && \
    echo 'git-fetch-with-cli = true' >> /root/.cargo/config.toml && \
    echo '' >> /root/.cargo/config.toml && \
    echo '[http]' >> /root/.cargo/config.toml && \
    echo 'timeout = 120' >> /root/.cargo/config.toml && \
    echo 'low-speed-limit = 5' >> /root/.cargo/config.toml && \
    echo 'multiplexing = false' >> /root/.cargo/config.toml && \
    echo 'check-revoke = false' >> /root/.cargo/config.toml && \
    echo '' >> /root/.cargo/config.toml && \
    echo '[build]' >> /root/.cargo/config.toml && \
    echo 'jobs = 2' >> /root/.cargo/config.toml

WORKDIR /usr/src/realtime-service

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo '#[cfg(test)] mod tests {}' >> src/lib.rs 2>/dev/null || true

RUN cargo build --release --target=x86_64-unknown-linux-musl || \
    (echo "Retry 1/3..." && sleep 10 && cargo build --release --target=x86_64-unknown-linux-musl) || \
    (echo "Retry 2/3..." && sleep 20 && cargo build --release --target=x86_64-unknown-linux-musl) || \
    (echo "Retry 3/3..." && sleep 30 && cargo build --release --target=x86_64-unknown-linux-musl)

RUN rm -rf src

COPY . .

RUN cargo build --release --target=x86_64-unknown-linux-musl


FROM alpine:latest

RUN apk add --no-cache ca-certificates

WORKDIR /root/

COPY --from=builder /usr/src/realtime-service/target/x86_64-unknown-linux-musl/release/realtime-service .

EXPOSE 8080

CMD ["./realtime-service"]