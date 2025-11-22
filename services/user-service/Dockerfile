FROM golang:1.25-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o user-service ./cmd/server

FROM golang:1.25-alpine AS development
WORKDIR /app

RUN go install github.com/air-verse/air@latest

RUN mkdir -p /app/tmp

COPY . .

RUN go mod download

EXPOSE 8002

CMD ["air", "-c", ".air.toml"]

FROM alpine:latest
WORKDIR /root/
COPY --from=builder /app/user-service .
ENTRYPOINT ["./user-service"]
CMD [""]