#!/usr/bin/env bash
set -euo pipefail

echo "[preflight] docker:"
docker --version >/dev/null

echo "[preflight] ports in use (80/443/8085/8795/8796):"
ss -ltnp | egrep ':(80|443|8085|8795|8796)\s' || true

echo "[preflight] disk:"
df -h /

echo "[preflight] state dirs:"
mkdir -p state/indexer state/oracle
chmod 777 state/indexer state/oracle || true

echo "[preflight] ok"


echo "[preflight] feature flags:" 
ls -lah state/feature_flags.json || true
