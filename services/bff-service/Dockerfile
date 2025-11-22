FROM golang:1.25-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o bff-service ./cmd/server

FROM golang:1.25-alpine AS development
WORKDIR /app
ENV PATH="/go/bin:/usr/local/go/bin:${PATH}"
RUN go install github.com/air-verse/air@latest
COPY . .
RUN go mod download
EXPOSE 8000
CMD ["air", "-c", ".air.toml"]

FROM alpine:latest
WORKDIR /root/
COPY --from=builder /app/bff-service .
ENTRYPOINT ["./bff-service"]
CMD [""]
