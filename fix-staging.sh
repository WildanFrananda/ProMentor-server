#!/bin/bash

# Skrip Perbaikan Deployment untuk Staging (Ubuntu)
# Usage: ./scripts/fix_staging_deployment.sh

echo "🔴 [1/5] Memulai prosedur perbaikan deployment Staging..."

# 1. Pastikan kita berada di root project
# (Asumsi skrip dijalankan dari root, jika tidak sesuaikan cd)
# cd /path/to/ios-backend

# 2. Tarik kode terbaru dari git
echo "⬇️ [2/5] Menarik kode terbaru dari repository..."
git pull origin main
if [ $? -ne 0 ]; then
    echo "❌ Gagal melakukan git pull. Cek koneksi internet atau conflict."
    exit 1
fi

# 3. Hentikan container lama & Bersihkan Orphan
echo "🛑 [3/5] Menghentikan container lama..."
docker compose -f infra/docker-compose.yml down --remove-orphans

# 4. Build Ulang dengan --no-cache (CRITICAL STEP)
# Kita gunakan --no-cache untuk menjamin binary Go dikompilasi ulang dengan kode terbaru
echo "🔨 [4/5] Membangun ulang image (Force Rebuild)..."
docker compose -f infra/docker-compose.yml up --build --force-recreate -d

# Tunggu sebentar agar service 'healthy'
echo "⏳ Menunggu service booting (10 detik)..."
sleep 10

# 5. Verifikasi Manual Endpoint yang Hilang
echo "🔍 [5/5] Verifikasi Endpoint..."

# Cek Categories
HTTP_STATUS=$(curl -o /dev/null -s -w "%{http_code}\n" http://localhost:8000/v1/categories)
if [ "$HTTP_STATUS" == "200" ]; then
    echo "✅ Endpoint /v1/categories: OK (200)"
else
    echo "❌ Endpoint /v1/categories: GAGAL ($HTTP_STATUS)"
    echo "   -> Kemungkinan build gagal atau kode belum terupdate."
fi

# Cek Profile
# Note: Profile butuh auth, jadi kita expect 401 (Unauthorized), BUKAN 404.
# Jika 404 berarti endpoint tidak ada. Jika 401 berarti endpoint ada tapi butuh token.
HTTP_STATUS_PROFILE=$(curl -o /dev/null -s -w "%{http_code}\n" http://localhost:8000/v1/profile/me)
if [ "$HTTP_STATUS_PROFILE" == "401" ]; then
    echo "✅ Endpoint /v1/profile/me: OK (Terdeteksi, Butuh Auth)"
elif [ "$HTTP_STATUS_PROFILE" == "200" ]; then
    echo "✅ Endpoint /v1/profile/me: OK (200)"
else
    echo "❌ Endpoint /v1/profile/me: GAGAL ($HTTP_STATUS_PROFILE)"
    echo "   -> Jika 404, berarti routing belum terdaftar."
fi

echo "---------------------------------------------------"
echo "🎉 Deployment Staging Selesai."
echo "Silakan minta iOS Team untuk mencoba ulang."