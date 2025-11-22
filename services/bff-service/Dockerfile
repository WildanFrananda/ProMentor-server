FROM golang:1.25-alpine AS builder

RUN apk add --no-cache ca-certificates git

WORKDIR /app

ENV GOPROXY=https://proxy.golang.org,https://goproxy.cn,direct
ENV GOSUMDB=sum.golang.org
ENV GONOPROXY=none
ENV GONOSUMDB=none
ENV GOPRIVATE=

COPY go.mod go.sum ./

RUN go mod download || \
    (sleep 5 && go mod download) || \
    (sleep 10 && go mod download) || \
    (sleep 15 && go mod download)

COPY . .

RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o bff-service ./cmd/server


FROM golang:1.25-alpine AS development

RUN apk add --no-cache ca-certificates git

WORKDIR /app

ENV PATH="/go/bin:/usr/local/go/bin:${PATH}"
ENV GOPROXY=https://proxy.golang.org,https://goproxy.cn,direct
ENV GOSUMDB=sum.golang.org

RUN go install github.com/air-verse/air@latest || \
    (sleep 5 && go install github.com/air-verse/air@latest)

COPY go.mod go.sum ./

RUN go mod download || \
    (sleep 5 && go mod download) || \
    (sleep 10 && go mod download)

COPY . .

EXPOSE 8000

CMD ["air", "-c", ".air.toml"]


# Production stage
FROM alpine:latest

RUN apk add --no-cache ca-certificates

WORKDIR /root/

COPY --from=builder /app/bff-service .

ENTRYPOINT ["./bff-service"]
CMD [""]