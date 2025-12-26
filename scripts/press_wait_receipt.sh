#!/usr/bin/env bash
set -euo pipefail
# Wait for transaction receipt
# Usage: press_wait_receipt.sh <rpc> <txhash>
RPC="${1:?rpc}"
TX="${2:?txhash}"
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
"${DIR}/_foundry.sh" cast receipt --rpc-url "${RPC}" "${TX}" >/dev/null
echo "ok"
