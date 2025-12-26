#!/usr/bin/env bash
set -euo pipefail

# Press Blockchain - Deploy (RC2)
# Run as pressblockchain user (recommended) after prerequisites are installed.

REPO_DIR="${1:-$(pwd)}"
cd "$REPO_DIR"

if [[ ! -f "config/.env" ]]; then
  echo "Missing config/.env"
  echo "Copy config/.env.example to config/.env and edit values."
  exit 1
fi

export $(grep -v '^#' config/.env | xargs -d '
' || true)
export COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-press-network-stack}"

echo "== Preflight =="
docker --version
docker compose version

echo "== Pull/Build =="
docker compose -f deploy/docker-compose.yml build

echo "== Up (all default profiles) =="
# Modules are controlled by compose profiles. This starts core + enabled profiles.
docker compose -f deploy/docker-compose.yml --profile oracle --profile status_page --profile dev_dapps --profile deployer --profile outlet_wizard --profile outlet_api --profile bots up -d

echo "== Status =="
docker compose -f deploy/docker-compose.yml ps
echo "OK: Deployed. Visit deployer UI on port ${DEPLOYER_UI_PORT:-3005}."
