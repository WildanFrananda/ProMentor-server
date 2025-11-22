# Tahap 1: Builder (Untuk Produksi)
FROM golang:1.25-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o auth-service ./cmd/server

# Tahap 2: Development Stage
FROM golang:1.25-alpine AS development
WORKDIR /app

# Install Air
RUN go install github.com/air-verse/air@latest

# Pastikan direktori tmp ada
RUN mkdir -p /app/tmp

# Copy source code
COPY . .

# Download dependencies
RUN go mod download

# Expose port
EXPOSE 8001

# Jalankan air
CMD ["air", "-c", ".air.toml"]

# Tahap Final: Produksi
FROM alpine:latest
WORKDIR /root/
COPY --from=builder /app/auth-service .
COPY --from=builder /app/migrations ./migrations
ENTRYPOINT ["./auth-service"]
CMD [""]