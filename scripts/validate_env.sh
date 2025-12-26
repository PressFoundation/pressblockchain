#!/usr/bin/env bash
set -euo pipefail
ENV_FILE="config/press.env"
REQ=(
  "CHAIN_ID"
  "CHAIN_NAME"
  "NATIVE_SYMBOL"
  "PUBLIC_RPC_HTTP"
  "EXPLORER_URL"
)
missing=0
for k in "${REQ[@]}"; do
  if ! grep -q "^${k}=" "$ENV_FILE"; then
    echo "[validate] missing $k"
    missing=1
  fi
done
if [ "$missing" -eq 1 ]; then
  echo "[validate] FAILED"
  exit 1
fi
echo "[validate] OK"
