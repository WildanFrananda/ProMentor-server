iOS Backend Monorepo
Repository ini berisi semua layanan backend yang mendukung aplikasi iOS. Arsitektur ini menggunakan pendekatan microservices dengan Go dan Rust, diatur dalam sebuah monorepo.

Layanan
Auth Service (Go/Fiber): Mengelola otentikasi pengguna (login, refresh token, JWT).

User Service (Go/Fiber): Mengelola data profil pengguna.

Realtime Service (Rust/Actix Web): Mengelola koneksi WebSocket untuk fitur real-time seperti chat dalam sesi.

Notification Worker (Go): Worker di latar belakang untuk mengirim push notification.

BFF (Backend for Frontend): Terintegrasi dalam layanan Go sebagai API Gateway internal.

Prasyarat
Docker & Docker Compose

Go (versi 1.21+)

Rust (toolchain stabil terbaru)

Menjalankan Lingkungan Lokal
Clone Repository

git clone <url-repo>
cd ios-backend

Siapkan Environment Variables
Salin file .env.example di setiap layanan menjadi .env.dev dan sesuaikan jika perlu.

cp services/auth-service/.env.example services/auth-service/.env.dev
# Lakukan untuk layanan lain...

Jalankan Semua Layanan
Gunakan Docker Compose untuk menjalankan semua infrastruktur dan layanan.

docker-compose -f infra/docker-compose.yml up --build -d

Perintah ini akan:

Mem-build image Docker untuk setiap layanan.

Menjalankan container untuk Postgres, Redis, NATS, dan semua layanan aplikasi.

Opsi -d menjalankannya di background (detached mode).

Struktur Direktori
services/: Kode sumber untuk setiap microservice.

infra/: Konfigurasi infrastruktur (Docker, Kubernetes).

docs/: Dokumentasi teknis (OpenAPI, skema event).

scripts/: Skrip bantuan untuk development.

# untuk minio access policy

Tambahkan service mc sementara ke docker-compose.yml (di bagian services:). Contoh snippet — cukup tambahkan (temporary):

``` yml
  mc:
    image: minio/mc
    container_name: ios_mc
    depends_on:
      - minio
    entrxypoint: ["sleep", "infinity"]   # container tetap hidup agar bisa exec ke dalamnya
```


Kemudian jalankan:
``` bash
docker compose -f infra/docker-compose.yml up -d mc
```

Masuk ke shell container mc (atau jalankan perintah langsung lewat docker compose exec)

# contoh: buat alias
``` bash
docker compose -f infra/docker-compose.yml exec mc mc alias set local http://minio:9000 minioadmin minioadmin
```
# buat bucket avatars
``` bash
docker compose -f infra/docker-compose.yml exec mc mc mb --ignore-existing local/avatars
```
# upload file hello.txt dari host (mount sementara)
``` bash
printf "hello via compose mc\n" > hello.txt
docker cp hello.txt ios_mc:/tmp/hello.txt
docker compose -f infra/docker-compose.yml exec mc mc cp /tmp/hello.txt local/avatars/hello.txt
docker compose -f infra/docker-compose.yml exec mc mc ls local/avatars

# set policy public read
docker compose -f infra/docker-compose.yml exec mc mc policy set download local/avatars

# cek policy
docker compose -f infra/docker-compose.yml exec mc mc policy info local/avatars
```

Verifikasi dari host
``` bash
curl -I http://localhost:9000/avatars/hello.txt
# atau http://localhost:9000/avatars/hello.txt
```

Hapus service mc setelah selesai
``` bash
docker compose -f infra/docker-compose.yml rm -sf mc
# atau
docker compose -f infra/docker-compose.yml down --remove-orphans
rm hello.txt
```

Run all service
```bash
docker compose -f infra/docker-compose.yml up postgres redis nats minio prometheus grafana jaeger 
```
```bash
docker compose -f infra/docker-compose.yml up auth-service user-service notification-worker
```
```bash
docker compose -f infra/docker-compose.yml up bff-service
```
```bash
docker compose -f infra/docker-compose.yml up realtime-service
```
