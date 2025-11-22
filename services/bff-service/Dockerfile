FROM golang:1.25-alpine AS development
RUN apk add --no-cache ca-certificates git
WORKDIR /app
ENV PATH="/go/bin:/usr/local/go/bin:${PATH}"

RUN go install github.com/air-verse/air@latest || \
    (sleep 5 && go install github.com/air-verse/air@latest)

# Copy everything including vendor
COPY . .

# Use vendor instead of downloading
RUN go mod vendor

EXPOSE 8000
CMD ["air", "-c", ".air.toml"]


# Production stage
FROM alpine:latest

RUN apk add --no-cache ca-certificates

WORKDIR /root/

COPY --from=builder /app/bff-service .

ENTRYPOINT ["./bff-service"]
CMD [""]