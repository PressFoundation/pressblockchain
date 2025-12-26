#!/usr/bin/env bash
set -euo pipefail
RPC_URL="${RPC_URL:-http://press-rpc:8545}"
STATE_DIR="${STATE_DIR:-state}"
DEPLOYER_KEY="${DEPLOYER_KEY:-}"

if [[ -z "$DEPLOYER_KEY" ]]; then
  echo "DEPLOYER_KEY is required"
  exit 1
fi

PRESS_TOKEN_ADDRESS="$(python3 -c 'import json;print(json.load(open("state/deploy.json")).get("pressToken","") or json.load(open("state/deploy.json")).get("press_token_address",""))' 2>/dev/null || true)"
TREASURY_ADDRESS="$(python3 -c 'import json;print(json.load(open("state/deploy.json")).get("treasury",""))' 2>/dev/null || true)"
PRESS_PARAMS_ADDRESS="$(python3 -c 'import json;print(json.load(open("state/deploy.json")).get("pressParameters",""))' 2>/dev/null || true)"

if [[ -z "$PRESS_TOKEN_ADDRESS" || -z "$TREASURY_ADDRESS" || -z "$PRESS_PARAMS_ADDRESS" ]]; then
  echo "Missing addresses in state/deploy.json (pressToken/treasury/pressParameters)"
  exit 1
fi

FOUNDRY_IMG="${FOUNDRY_IMG:-ghcr.io/foundry-rs/foundry:latest}"

echo "[deploy] compiling..."
docker run --rm -i -v "$(pwd):/w" -w /w "$FOUNDRY_IMG" bash -lc "forge build"

echo "[deploy] ExchangeListingRegistry..."
OUT="$(docker run --rm -i -v "$(pwd):/w" -w /w "$FOUNDRY_IMG" bash -lc "forge create contracts/src/ExchangeListingRegistry.sol:ExchangeListingRegistry --rpc-url '$RPC_URL' --private-key '$DEPLOYER_KEY' --constructor-args $PRESS_TOKEN_ADDRESS $TREASURY_ADDRESS $PRESS_PARAMS_ADDRESS --json")"
ADDR="$(python3 -c 'import json,sys;print(json.loads(sys.argv[1]).get("deployedTo",""))' "$OUT")"

if [[ -z "$ADDR" ]]; then
  echo "Deploy failed."
  exit 1
fi

python3 - <<PY
import json
p="state/deploy.json"
d=json.load(open(p))
d["exchangeListingRegistry"]= "$ADDR"
json.dump(d, open(p,"w"), indent=2)
print("[ok] wrote exchangeListingRegistry to", p)
PY

echo "[ok] deployed ExchangeListingRegistry: $ADDR"
