#!/usr/bin/env bash
set -euo pipefail

# Verify DNS points subdomains to BIND_IP
REPO_DIR="${1:-$(pwd)}"
cd "$REPO_DIR"
set -a
source config/.env
set +a

IP="${BIND_IP:-38.146.25.37}"
SUBS=("deploy" "status" "rpc" "explorer" "wallet" "bots")

ok=1
for s in "${SUBS[@]}"; do
  d="${s}.pressblockchain.io"
  a=$(dig +short A "$d" | tail -n1 || true)
  if [[ "$a" != "$IP" ]]; then
    echo "FAIL  $d -> $a (expected $IP)"
    ok=0
  else
    echo "OK    $d -> $a"
  fi
done

if [[ "$ok" -ne 1 ]]; then
  exit 2
fi
echo "All DNS checks passed."
