#!/usr/bin/env bash
set -euo pipefail

# Deploy an OutletToken using Foundry (in-container).
# Required env:
#   RPC_URL, PRIVATE_KEY, TOKEN_NAME, TOKEN_SYMBOL, TOTAL_SUPPLY_WEI, OWNER_ADDR, LIMITS_ENABLED, MAX_TX, MAX_WALLET

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ -z "${RPC_URL:-}" || -z "${PRIVATE_KEY:-}" ]]; then
  echo "Missing RPC_URL or PRIVATE_KEY" >&2
  exit 2
fi

# Create minimal foundry project layout if missing
mkdir -p "${ROOT_DIR}/.foundry/src" "${ROOT_DIR}/.foundry/lib" "${ROOT_DIR}/.foundry/out"
cp -f "${ROOT_DIR}/contracts/outlet_token/OutletToken.sol" "${ROOT_DIR}/.foundry/src/OutletToken.sol"

# init foundry config
cat > "${ROOT_DIR}/.foundry/foundry.toml" <<'TOML'
[profile.default]
src = "src"
out = "out"
libs = ["lib"]
solc_version = "0.8.23"
optimizer = true
optimizer_runs = 200
TOML

# Install OZ if missing
if [[ ! -d "${ROOT_DIR}/.foundry/lib/openzeppelin-contracts" ]]; then
  (cd "${ROOT_DIR}/.foundry" && forge install OpenZeppelin/openzeppelin-contracts --no-commit)
fi

(cd "${ROOT_DIR}/.foundry" && forge build)

DEPLOY_OUTPUT=$(
  cd "${ROOT_DIR}/.foundry" &&   forge create src/OutletToken.sol:OutletToken     --rpc-url "${RPC_URL}"     --private-key "${PRIVATE_KEY}"     --constructor-args "${TOKEN_NAME}" "${TOKEN_SYMBOL}" "${TOTAL_SUPPLY_WEI}" "${OWNER_ADDR}" "${LIMITS_ENABLED}" "${MAX_TX}" "${MAX_WALLET}"
)

# parse deployed address
ADDR=$(echo "${DEPLOY_OUTPUT}" | awk '/Deployed to:/ {print $3}' | tail -n 1)
TX=$(echo "${DEPLOY_OUTPUT}" | awk '/Transaction hash:/ {print $3}' | tail -n 1)

if [[ -z "${ADDR}" ]]; then
  echo "Failed to parse deployed address" >&2
  echo "${DEPLOY_OUTPUT}" >&2
  exit 3
fi

echo "${ADDR}|${TX}"
