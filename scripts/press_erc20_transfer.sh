#!/usr/bin/env bash
set -euo pipefail
# Transfer ERC20 tokens using cast (containerized).
# Usage: press_erc20_transfer.sh <rpc> <token> <from_privkey> <to> <amount_wei>
RPC="${1:?rpc}"
TOKEN="${2:?token}"
PK="${3:?privkey}"
TO="${4:?to}"
AMT="${5:?amount_wei}"

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
"${DIR}/_foundry.sh" cast send --rpc-url "${RPC}" --private-key "${PK}" "${TOKEN}" "transfer(address,uint256)" "${TO}" "${AMT}"
