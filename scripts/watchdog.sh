#!/usr/bin/env bash
set -euo pipefail
cd /opt/pressblockchain/press-blockchain

echo "[watchdog] $(date -Is) checking ops health..."

OPS_URL="http://127.0.0.1:8081/ops/health"

if ! curl -fsS "$OPS_URL" >/dev/null 2>&1; then
  echo "[watchdog] ops endpoint unreachable - restarting compose stack"
  docker compose --env-file config/press.env restart
  exit 0
fi

OK=$(curl -fsS "$OPS_URL" | python3 -c "import sys, json; print('1' if json.load(sys.stdin).get('ok') else '0')")
if [ "$OK" != "1" ]; then
  echo "[watchdog] ops reports degraded - restarting core services"
  docker compose --env-file config/press.env restart press-rpc press-gateway-api || docker compose --env-file config/press.env restart
fi

echo "[watchdog] ok"
