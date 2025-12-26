#!/usr/bin/env bash
set -euo pipefail

# Press Blockchain - Deterministic Deploy (RC4)
# Run as pressblockchain user.

REPO_DIR="${1:-$(pwd)}"
cd "$REPO_DIR"

if [[ ! -f "config/.env" ]]; then
  echo "Missing config/.env"
  echo "Copy config/.env.example to config/.env and edit values."
  exit 1
fi

set -a
source config/.env
set +a

export COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-press-network-stack}"

step() { echo; echo "== $1 =="; }
fail() { echo; echo "FAILED: $1"; exit 1; }

step "Preflight"
docker --version >/dev/null || fail "docker missing"
docker compose version >/dev/null || fail "docker compose missing"
ss -ltnp | head -n 80 || true

step "Build"
docker compose -f deploy/docker-compose.yml build

step "Core: RPC up"
docker compose -f deploy/docker-compose.yml up -d press-rpc || fail "rpc up failed"
echo "Waiting for RPC health..."
for i in $(seq 1 90); do
  if curl -s -X POST -H 'Content-Type: application/json' --data '{"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]}' http://127.0.0.1:${RPC_PORT:-8545} | grep -q "result"; then
    echo "RPC OK"; break
  fi
  sleep 1
  [[ $i -eq 90 ]] && fail "rpc healthcheck timeout"
done

step "Core: Validator up"
docker compose -f deploy/docker-compose.yml up -d press-validator || fail "validator up failed"

step "Core: Deployer + Status"
docker compose -f deploy/docker-compose.yml --profile deployer --profile status_page up -d || fail "deployer/status up failed"

step "Outlet stack (wizard + api)"
docker compose -f deploy/docker-compose.yml --profile outlet_wizard --profile outlet_api up -d || fail "outlet stack failed"

step "Optional profiles (enable via compose profiles)"
docker compose -f deploy/docker-compose.yml --profile oracle --profile dev_dapps --profile bots up -d || true

step "Final status"
docker compose -f deploy/docker-compose.yml ps
echo "OK: Deterministic deploy finished."
echo "Deployer UI: http://$(hostname -I | awk '{print $1}'):${DEPLOYER_UI_PORT:-3005}"
echo "Status UI:   http://$(hostname -I | awk '{print $1}'):${STATUS_UI_PORT:-3007}"
