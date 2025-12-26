#!/usr/bin/env bash
set -euo pipefail
REPO_DIR="${1:-$(pwd)}"
cd "$REPO_DIR"

if [[ ! -f "config/.env" ]]; then echo "Missing config/.env"; exit 1; fi
set -a; source config/.env; set +a
export COMPOSE_PROJECT_NAME="${COMPOSE_PROJECT_NAME:-press-network-stack}"

PROFILES=()
if [[ -f "config/modules.json" ]]; then
  for k in $(jq -r 'keys[]' config/modules.json); do
    v=$(jq -r --arg k "$k" '.[$k]' config/modules.json)
    if [[ "$v" == "true" ]]; then PROFILES+=("--profile" "$k"); fi
  done
fi

docker compose -f deploy/docker-compose.yml build
docker compose -f deploy/docker-compose.yml up -d press-rpc
for i in $(seq 1 90); do
  if curl -s -X POST -H 'Content-Type: application/json' --data '{"jsonrpc":"2.0","id":1,"method":"eth_chainId","params":[]}' http://127.0.0.1:${RPC_PORT:-8545} | grep -q "result"; then break; fi
  sleep 1; [[ $i -eq 90 ]] && exit 2
done
docker compose -f deploy/docker-compose.yml up -d press-validator
if [[ ${#PROFILES[@]} -gt 0 ]]; then
  docker compose -f deploy/docker-compose.yml "${PROFILES[@]}" up -d
fi
docker compose -f deploy/docker-compose.yml ps
