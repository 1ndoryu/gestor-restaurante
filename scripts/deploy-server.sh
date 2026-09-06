#!/bin/bash
# [263A-22] Script para deploy del backend en el servidor
# Reconstruye imagen Docker y arranca contenedores con Traefik
# Requiere: DB_PASSWORD y JWT_SECRET como variables de entorno

set -e

if [ -z "$DB_PASSWORD" ] || [ -z "$JWT_SECRET" ]; then
  echo "ERROR: DB_PASSWORD y JWT_SECRET deben estar definidos como variables de entorno"
  exit 1
fi

CONTAINER_NAME="app-b8s0cks444o0sogo8kg8wcgw"
PG_CONTAINER="postgres-b8s0cks444o0sogo8kg8wcgw"
NETWORK="b8s0cks444o0sogo8kg8wcgw"
IMAGE="glory-rest-app:latest"
DOMAIN="app-b8s0cks444o0sogo8kg8wcgw.66.94.100.241.sslip.io"
REPO_URL="https://github.com/1ndoryu/glory-rs.git"
BRANCH="main"
BUILD_DIR="/tmp/glory-rs-build"

# Reconstruir imagen
echo "Clonando y construyendo imagen..."
rm -rf "$BUILD_DIR"
git clone --branch "$BRANCH" --depth 1 --recurse-submodules "$REPO_URL" "$BUILD_DIR"
cd "$BUILD_DIR"
docker build -t "$IMAGE" .

# Parar y eliminar contenedor anterior
docker stop "$CONTAINER_NAME" 2>/dev/null || true
docker rm "$CONTAINER_NAME" 2>/dev/null || true

# Arrancar app
docker run -d \
  --name "$CONTAINER_NAME" \
  --network "$NETWORK" \
  --restart unless-stopped \
  -p 3001:3000 \
  -e "DATABASE_URL=postgres://glory_app:${DB_PASSWORD}@${PG_CONTAINER}:5432/glory" \
  -e "JWT_SECRET=${JWT_SECRET}" \
  -e "HOST=0.0.0.0" \
  -e "PORT=3000" \
  -e "STATIC_DIR=/app/static" \
  -e "DEMO_MODE=true" \
  -e "CORS_ORIGINS=http://restaurante.wandori.us" \
  -l "traefik.enable=true" \
  -l "traefik.http.routers.http-0-${NETWORK}-app.entryPoints=http" \
  -l "traefik.http.routers.http-0-${NETWORK}-app.rule=Host(\`${DOMAIN}\`) && PathPrefix(\`/\`)" \
  -l "traefik.http.services.http-0-${NETWORK}-app.loadbalancer.server.port=3000" \
  -l "coolify.managed=true" \
  "$IMAGE"

# Conectar a la red coolify para que Traefik lo vea
docker network connect coolify "$CONTAINER_NAME" 2>/dev/null || true

echo "Glory REST app deployed successfully"
echo "Direct: http://66.94.100.241:3001/swagger-ui/"
echo "Traefik: http://${DOMAIN}/swagger-ui/"
