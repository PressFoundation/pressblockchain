#!/usr/bin/env bash
set -euo pipefail

# Deploy OutletRegistry + OutletTokenFactory using foundry in a container.
# Requires:
# - RPC_URL (default http://press-rpc:8545)
# - DEPLOYER_KEY (hex private key)
# - PRESS_TOKEN_ADDRESS (from state/press_token_address.txt or deploy.json)
# - TREASURY_ADDRESS (from state/treasury_address.txt or deploy.json)
# - PRESS_PARAMS_ADDRESS (from state/deploy.json)
#
# Writes into: state/deploy.json (adds outletRegistry, outletTokenFactory)

RPC_URL="${RPC_URL:-http://press-rpc:8545}"
STATE_DIR="${STATE_DIR:-state}"
DEPLOYER_KEY="${DEPLOYER_KEY:-}"
PRESS_TOKEN_ADDRESS="${PRESS_TOKEN_ADDRESS:-}"
TREASURY_ADDRESS="${TREASURY_ADDRESS:-}"
PRESS_PARAMS_ADDRESS="${PRESS_PARAMS_ADDRESS:-}"

if [[ -z "$DEPLOYER_KEY" ]]; then
  echo "DEPLOYER_KEY is required (hex privkey)."
  exit 1
fi

# best-effort auto-detect
if [[ -z "$PRESS_TOKEN_ADDRESS" && -f "$STATE_DIR/press_token_address.txt" ]]; then
  PRESS_TOKEN_ADDRESS="$(cat "$STATE_DIR/press_token_address.txt" | tr -d '
' )"
fi

if [[ -z "$PRESS_PARAMS_ADDRESS" && -f "$STATE_DIR/deploy.json" ]]; then
  PRESS_PARAMS_ADDRESS="$(python3 -c 'import json;print(json.load(open("state/deploy.json")).get("pressParameters",""))')"
fi

if [[ -z "$TREASURY_ADDRESS" && -f "$STATE_DIR/deploy.json" ]]; then
  TREASURY_ADDRESS="$(python3 -c 'import json;print(json.load(open("state/deploy.json")).get("treasury",""))')"
fi

if [[ -z "$PRESS_TOKEN_ADDRESS" || -z "$TREASURY_ADDRESS" || -z "$PRESS_PARAMS_ADDRESS" ]]; then
  echo "Missing addresses. PRESS_TOKEN_ADDRESS=$PRESS_TOKEN_ADDRESS TREASURY_ADDRESS=$TREASURY_ADDRESS PRESS_PARAMS_ADDRESS=$PRESS_PARAMS_ADDRESS"
  exit 1
fi

echo "[deploy] using:"
echo "RPC_URL=$RPC_URL"
echo "PRESS_TOKEN=$PRESS_TOKEN_ADDRESS"
echo "TREASURY=$TREASURY_ADDRESS"
echo "PRESS_PARAMS=$PRESS_PARAMS_ADDRESS"

# Use foundry docker image
FOUNDRY_IMG="${FOUNDRY_IMG:-ghcr.io/foundry-rs/foundry:latest}"

deploy_contract() {
  local contract_path="$1"
  local constructor_args="$2"
  docker run --rm -i     -v "$(pwd):/w" -w /w     -e RUST_LOG=info     "$FOUNDRY_IMG"     bash -lc "forge create $contract_path --rpc-url '$RPC_URL' --private-key '$DEPLOYER_KEY' --constructor-args $constructor_args --json"
}

# We assume foundry project is at repo root contracts/; compile uses forge inside container
echo "[deploy] compiling..."
docker run --rm -i -v "$(pwd):/w" -w /w "$FOUNDRY_IMG" bash -lc "forge build"

echo "[deploy] OutletRegistry..."
OUTLET_REG_JSON="$(deploy_contract contracts/src/OutletRegistry.sol:OutletRegistry "$PRESS_TOKEN_ADDRESS $TREASURY_ADDRESS $PRESS_PARAMS_ADDRESS")"
OUTLET_REG_ADDR="$(python3 -c 'import json,sys;print(json.loads(sys.argv[1]).get("deployedTo",""))' "$OUTLET_REG_JSON")"

echo "[deploy] OutletTokenFactory..."
OUTLET_FAC_JSON="$(deploy_contract contracts/src/OutletTokenFactory.sol:OutletTokenFactory "$PRESS_TOKEN_ADDRESS $TREASURY_ADDRESS $PRESS_PARAMS_ADDRESS")"
OUTLET_FAC_ADDR="$(python3 -c 'import json,sys;print(json.loads(sys.argv[1]).get("deployedTo",""))' "$OUTLET_FAC_JSON")"

if [[ -z "$OUTLET_REG_ADDR" || -z "$OUTLET_FAC_ADDR" ]]; then
  echo "Deploy failed. Registry=$OUTLET_REG_ADDR Factory=$OUTLET_FAC_ADDR"
  exit 1
fi

python3 - <<PY
import json
p="state/deploy.json"
d=json.load(open(p))
d["outletRegistry"]= "$OUTLET_REG_ADDR"
d["outletTokenFactory"]= "$OUTLET_FAC_ADDR"
json.dump(d, open(p,"w"), indent=2)
print("[ok] wrote outletRegistry/outletTokenFactory to", p)
PY

echo "[ok] Outlet contracts deployed."
