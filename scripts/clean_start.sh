#!/usr/bin/env bash
set -euo pipefail

# Press Blockchain - Clean Start
# Removes project containers/networks/volumes for a deterministic redeploy.

REPO_DIR="${1:-$(pwd)}"
cd "$REPO_DIR"

export $(grep -v '^#' config/.env | xargs -d '
' || true)
export COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-press-network-stack}"

docker compose -f deploy/docker-compose.yml down -v --remove-orphans || true

# Remove any stray run containers matching project prefix
docker ps -a --format '{{.Names}}' | grep -E "^${COMPOSE_PROJECT_NAME}-.*-run-" | xargs -r docker rm -f || true

echo "OK: Clean start complete."
