#!/usr/bin/env bash
set -euo pipefail

echo "[auto-fix] Checking Docker..."
command -v docker >/dev/null 2>&1 || { echo "Docker missing"; exit 1; }

echo "[auto-fix] Checking ports 80/443..."
ss -ltnp | grep -E ':(80|443) ' || true

echo "[auto-fix] Compose cleanup (safe)..."
export COMPOSE_PROJECT_NAME=${COMPOSE_PROJECT_NAME:-press-network-stack}
docker compose down -v --remove-orphans || true
docker ps -a --format '{{.Names}}' | grep "${COMPOSE_PROJECT_NAME}.*-run-" | xargs -r docker rm -f || true

echo "[auto-fix] Network recreate..."
docker network rm "${COMPOSE_PROJECT_NAME}_default" >/dev/null 2>&1 || true

echo "[auto-fix] Done. You can retry deployment from the installer."
