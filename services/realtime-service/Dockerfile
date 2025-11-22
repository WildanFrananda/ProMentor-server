# Tahap 1: Builder dengan target MUSL
# PERBAIKAN: Menggunakan versi Rust yang lebih baru untuk mendukung Edition 2024
FROM rust:1.85 AS builder

# Menambahkan target musl
RUN rustup target add x86_64-unknown-linux-musl

# PERBAIKAN: Menginstal C toolchain yang dibutuhkan untuk kompilasi musl
RUN apt-get update && apt-get install -y musl-tools

WORKDIR /usr/src/realtime-service
COPY . .

# Meng-compile binary secara statis untuk target musl
RUN cargo build --release --target=x86_64-unknown-linux-musl


# Tahap 2: Final Image yang sangat ringan
# Menggunakan Alpine karena binary musl kompatibel dengannya
FROM alpine:latest

WORKDIR /root/

# Menyalin binary yang sudah di-compile dari stage builder
COPY --from=builder /usr/src/realtime-service/target/x86_64-unknown-linux-musl/release/realtime-service .

EXPOSE 8080

# Menjalankan binary
CMD ["./realtime-service"]

