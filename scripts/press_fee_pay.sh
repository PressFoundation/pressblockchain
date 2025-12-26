#!/usr/bin/env bash
set -euo pipefail
# Pay a PRESS-denominated fee by transferring PRESS to treasury and waiting for receipt.
# Usage: press_fee_pay.sh <rpc> <press_token> <from_privkey> <treasury> <amount_press>
RPC="${1:?rpc}"
PRESS_TOKEN="${2:?press_token}"
PK="${3:?privkey}"
TREASURY="${4:?treasury}"
AMT_PRESS="${5:?amount_press}"

WEI=$(python3 - <<PY
from decimal import Decimal
amt=Decimal("${AMT_PRESS}")
print(int(amt*(10**18)))
PY
)

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TX=$("${DIR}/press_erc20_transfer.sh" "${RPC}" "${PRESS_TOKEN}" "${PK}" "${TREASURY}" "${WEI}" | tail -n 1)
"${DIR}/press_wait_receipt.sh" "${RPC}" "${TX}"
echo "${TX}"
