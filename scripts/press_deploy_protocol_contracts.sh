#!/usr/bin/env bash
set -euo pipefail
# Deploy Press protocol helper contracts via Foundry container.
# Usage: press_deploy_protocol_contracts.sh <rpc> <deployer_privkey> <press_token> <protocol_fee_bps> <vote_fee_press> <publish_fee_press> <coauthor_fee_press> <import_fee_press>
RPC="${1:?rpc}"
PK="${2:?privkey}"
PRESS_TOKEN="${3:?press_token}"
BPS="${4:?bps}"
VOTE_F="${5:?vote_fee_press}"
PUB_F="${6:?publish_fee_press}"
CO_F="${7:?coauthor_fee_press}"
IMP_F="${8:?import_fee_press}"

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${DIR}/.." && pwd)"
C_DIR="${ROOT_DIR}/contracts/press-protocol"

to_wei () {
  python3 - <<PY
from decimal import Decimal
amt=Decimal("$1")
print(int(amt*(10**18)))
PY
}

export PRESS_TOKEN
export TREASURY_OWNER=0x0000000000000000000000000000000000000001
export PROTOCOL_FEE_BPS="${BPS}"
export FEE_VOTE_WEI="$(to_wei "${VOTE_F}")"
export FEE_PUBLISH_WEI="$(to_wei "${PUB_F}")"
export FEE_COAUTHOR_WEI="$(to_wei "${CO_F}")"
export FEE_IMPORT_WEI="$(to_wei "${IMP_F}")"

docker run --rm --network host -e PRESS_TOKEN -e TREASURY_OWNER -e PROTOCOL_FEE_BPS -e FEE_VOTE_WEI -e FEE_PUBLISH_WEI -e FEE_COAUTHOR_WEI -e FEE_IMPORT_WEI   -v "${C_DIR}:/ws" -w /ws ghcr.io/foundry-rs/foundry:latest   forge script script/DeployPressProtocol.s.sol:DeployPressProtocol --rpc-url "${RPC}" --private-key "${PK}" --broadcast -vvvv
