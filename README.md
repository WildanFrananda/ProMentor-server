# iOS Backend Monorepo

Welcome to the backend repository for the **iOS SuperApp** project. This project uses a **Microservices** architecture organized as a **Monorepo**.

The system is built with modern technologies for high performance and scalability:

* **Go (Fiber)** — For RESTful services (Auth, User, BFF).
* **Rust (Actix)** — For high-performance Realtime service (WebSocket).
* **PostgreSQL** — Primary relational database.
* **Redis** — Fast caching and session management.
* **NATS** — Message broker for event-driven communication between services.
* **MinIO** — Object storage (S3 compatible) for static files and user avatars.

---

📋 **Table of Contents**

* [System Architecture](#system-architecture)
* [Prerequisites](#prerequisites)
* [Quick Start (Run Locally)](#quick-start-run-locally)
* [Project Structure](#project-structure)
* [Configuration Management (.env)](#configuration-management-env)
* [Development Workflow](#development-workflow)

  * [Database Migrations](#database-migrations)
  * [Adding Dependencies](#adding-dependencies)
  * [MinIO (S3) Setup](#minio-s3-setup)
  * [Hot Reload](#hot-reload)
* [Observability (Monitoring & Tracing)](#observability-monitoring--tracing)
* [Ports List](#ports-list)

---

# System Architecture

This system uses the **BFF (Backend for Frontend)** pattern. The client (iOS App) **ONLY** talks to the `bff-service`, which acts as the main gateway.

```mermaid
flowchart TD
    Client[iOS App] -->|HTTP :8000| BFF[BFF Service]
    Client -->|WS :8080| RT[Realtime Service]

    BFF -->|HTTP/Auth| Auth[Auth Service]
    BFF -->|HTTP/User| User[User Service]

    Auth --> DB[(Postgres)]
    User --> DB
    User --> MinIO[(MinIO S3)]

    Auth --Event--> NATS((NATS))
    RT --Sub--> NATS
    Worker[Notification Worker] --Sub--> NATS
```

**Main services:**

* **bff-service**: Primary gateway. Handles proxying, data aggregation, and rate limiting.
* **auth-service**: Handles registration, login, refresh token, session management, ratings, and chat history.
* **user-service**: Handles user profiles, device tokens, and avatar uploads to S3.
* **realtime-service**: Manages WebSocket connections, realtime chat, and broadcasting events to clients.
* **notification-worker**: Background worker that sends Push Notifications (APNs) based on events from NATS.

---

# Prerequisites

Before starting, make sure the following tools are installed on your machine:

* **Docker & Docker Compose (Required)** — The whole environment runs in containers.
* **Go (Version 1.25+)** — For local development of Go services.
* **Rust (Version 1.79+)** — If you work on the `realtime-service`.
* **Postman / API Client** — For API testing.

---

# Quick Start (Run Locally)

Follow these steps to run the entire system from scratch:

## 1. Clone Repository

```bash
git clone <your-repo-url>
cd ios-backend
```

## 2. Setup Environment Variables

Copy the example `.env.example` to `.env.dev` in each service directory.

```bash
# Auth Service
cp services/auth-service/.env.example services/auth-service/.env.dev

# User Service
cp services/user-service/.env.example services/user-service/.env.dev

# BFF Service
cp services/bff-service/.env.example services/bff-service/.env.dev

# Realtime Service
cp services/realtime-service/.env.example services/realtime-service/.env.dev

# Notification Worker
cp services/notification-worker/.env.example services/notification-worker/.env.dev
```

**IMPORTANT:** Ensure the `INTERNAL_SHARED_SECRET` variable is set to the **same** random string in `auth-service`, `user-service`, and `bff-service` for secure internal communication.

## 3. Start Services with Docker Compose

This command will build images, start containers, and enable hot-reload (Air for Go).

```bash
docker compose -f infra/docker-compose.yml up --build
```

Wait until all services report **"Healthy"** or logs show servers running and connected to NATS/DB.

## 4. Run Database Migrations

Apply the latest DB schema to PostgreSQL:

```bash
docker compose -f infra/docker-compose.yml run --build --rm auth-service-migrate
```

The system is now ready! Access the API through the BFF at: `http://localhost:8000`.

---

# Project Structure

```
ios-backend/
├── infra/                  # Infrastructure configuration (Docker, Prometheus)
│   ├── docker-compose.yml  # Local orchestration
│   └── prometheus/         # Monitoring config
├── services/               # Microservices source code
│   ├── auth-service/       # Go Fiber (Auth, Session, Rating, Chat History)
│   │   ├── cmd/server/     # Entry point
│   │   ├── internal/       # Business logic (API, Service, Repo, Events)
│   │   ├── migrations/     # DB migration files (Goose)
│   │   └── tracing/        # OpenTelemetry setup
│   ├── user-service/       # Go Fiber (Profile & S3 Integration)
│   ├── bff-service/        # Go Fiber (Gateway, Proxy, Aggregation)
│   ├── realtime-service/   # Rust Actix (WebSocket & Chat)
│   └── notification-worker/# Go Worker (NATS Consumer & APNs)
└── README.md
```

---

# Configuration Management (.env)

Each service has its own `.env.dev` file. Key variables to pay attention to:

* **Database:** `DB_HOST`, `DB_PORT`, `DB_USER`, `DB_PASSWORD` (Default: connects to container `ios_postgres`).
* **JWT:** `JWT_SECRET` (Must be the same across `auth-service`, `user-service`, and `realtime-service` for token validation).
* **Internal Security:** `INTERNAL_SHARED_SECRET` (Secret token for inter-service HTTP communication).
* **MinIO (S3):** `S3_ENDPOINT` should be set to `http://localhost:9000` so image URLs are accessible to clients outside Docker. `S3_USE_PATH_STYLE=true` is required for MinIO.

---

# Development Workflow

## Database Migrations

We use **Goose** for DB migrations. Migration files live in `services/auth-service/migrations`.

**To create a new migration:**
Create a new `.go` file in the `migrations` folder with a sequential name (e.g., `00008_add_new_table.go`).

**To run migrations:**

```bash
docker compose -f infra/docker-compose.yml run --build --rm auth-service-migrate
```

## Adding Dependencies

* **Go:** Run `go get <package-name>` in the respective service directory, then run `go mod tidy`. Restart Docker Compose with `--build`.
* **Rust:** Add dependency in `Cargo.toml`, then restart Docker Compose with `--build`.

# MinIO (S3) Setup

To enable avatar upload and public access to files on your local MinIO, follow these steps.

1. Open the MinIO Console: `http://localhost:9001` (User/Pass: `minioadmin`).
2. Create a bucket named `avatars`.
3. Ensure these env vars in your service `.env.dev` are set:

   * `S3_ENDPOINT=http://localhost:9000`
   * `S3_USE_PATH_STYLE=true`

---

## (Optional, recommended) — Set access policy using `mc` (MinIO Client) via a temporary container

If you want to script/set the bucket policy to public/readable from the host, add a temporary `mc` service to `infra/docker-compose.yml` under `services:`. Example snippet (temporary):

```yml
  mc:
    image: minio/mc
    container_name: ios_mc
    depends_on:
      - minio
    entrypoint: ["sleep", "infinity"]   # keep container alive so we can exec into it
```

Then bring up only that `mc` container:

```bash
docker compose -f infra/docker-compose.yml up -d mc
```

### Use `mc` to configure the bucket (examples)

Create an alias pointing to the MinIO service inside the compose network:

```bash
docker compose -f infra/docker-compose.yml exec mc mc alias set local http://minio:9000 minioadmin minioadmin
```

Create the `avatars` bucket (idempotent with `--ignore-existing`):

```bash
docker compose -f infra/docker-compose.yml exec mc mc mb --ignore-existing local/avatars
```

Upload a small test file from the host (we copy it into the mc container first):

```bash
printf "hello via compose mc\n" > hello.txt
docker cp hello.txt ios_mc:/tmp/hello.txt
docker compose -f infra/docker-compose.yml exec mc mc cp /tmp/hello.txt local/avatars/hello.txt
docker compose -f infra/docker-compose.yml exec mc mc ls local/avatars
```

Set the bucket policy to public download (read):

```bash
docker compose -f infra/docker-compose.yml exec mc mc policy set download local/avatars
```

Check the policy info:

```bash
docker compose -f infra/docker-compose.yml exec mc mc policy info local/avatars
```

Verify from the host (should return file content / HTTP headers):

```bash
curl -I http://localhost:9000/avatars/hello.txt
# or
curl http://localhost:9000/avatars/hello.txt
```

### Clean up the temporary `mc` service when done

```bash
# remove the mc container (force, stop first if needed)
docker compose -f infra/docker-compose.yml rm -sf mc

# or take the compose down and remove orphans (if you prefer)
docker compose -f infra/docker-compose.yml down --remove-orphans

# cleanup the test file on host
rm hello.txt
```

---

### Notes & Troubleshooting

* The `mc` alias command uses the service name `minio` (as defined in your `docker-compose.yml`) — if your MinIO service has a different name, adjust the alias URL accordingly.
* `S3_ENDPOINT` in your services should be `http://localhost:9000` so clients outside Docker can access uploaded objects.
* `S3_USE_PATH_STYLE=true` is required for MinIO compatibility.
* If `curl` returns a 403, double-check the bucket policy and that `mc policy set download` was applied to the correct bucket.
* You only need the `mc` container temporarily — it’s safe to remove it after configuring the bucket and policy.

If you want, I can merge this updated MinIO setup section into the README text you asked me to translate earlier and show you the exact updated README snippet. Which would you prefer?


## Hot Reload

All Go services use **Air**. Save files in your editor (Ctrl+S) and the server inside the container will automatically rebuild and restart.

---

# Observability (Monitoring & Tracing)

The system includes a local monitoring stack:

* **Grafana:** `http://localhost:3000` (Login: `admin/admin`) — Dashboards to visualize metrics from all services.
* **Prometheus:** `http://localhost:9090` — Metrics collector. Each Go and Rust service exposes `/metrics`.
* **Jaeger:** `http://localhost:16686` — Distributed tracing. Use this to trace the lifecycle of a request from BFF → Auth → DB, including latency at each step.

---

# Ports List

| Service          | Port (Host) | Description                    |
| ---------------- | ----------- | ------------------------------ |
| BFF Service      | 8000        | Main API entry point (HTTP)    |
| Auth Service     | 8001        | Internal API (Auth, Session)   |
| User Service     | 8002        | Internal API (Profile)         |
| Realtime Service | 8080        | WebSocket entry point          |
| Postgres         | 5432        | Primary database               |
| Redis            | 6379        | Cache                          |
| NATS             | 4222        | Message Broker / Event Bus     |
| MinIO API        | 9000        | S3 API (Upload/Download files) |
| MinIO Console    | 9001        | Storage admin UI               |
| Grafana          | 3000        | Monitoring dashboard UI        |
| Prometheus       | 9090        | Metrics query UI               |
| Jaeger           | 16686       | Distributed tracing UI         |

---

**Note:** This document was last updated for **Phase 3 (Production Readiness)**. If you encounter issues running the project, check the container logs:

```bash
docker compose logs -f <service_name>
```

or contact the **Tech Lead**.
