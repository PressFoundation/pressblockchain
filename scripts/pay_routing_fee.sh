#!/usr/bin/env bash
set -euo pipefail
# Pay routing fee in PRESS: transfer from payer -> treasury.
# Required env: RPC_URL, PAYER_PRIVATE_KEY, PRESS_TOKEN_ADDR, TREASURY_ADDR, AMOUNT_WEI
if [[ -z "${RPC_URL:-}" || -z "${PAYER_PRIVATE_KEY:-}" || -z "${PRESS_TOKEN_ADDR:-}" || -z "${TREASURY_ADDR:-}" || -z "${AMOUNT_WEI:-}" ]]; then
  echo "Missing env vars" >&2
  exit 2
fi
TXHASH=$(cast send "${PRESS_TOKEN_ADDR}" "transfer(address,uint256)(bool)" "${TREASURY_ADDR}" "${AMOUNT_WEI}"   --rpc-url "${RPC_URL}" --private-key "${PAYER_PRIVATE_KEY}" | awk '/transactionHash/ {print $2}' | tail -n 1)
if [[ -z "${TXHASH}" ]]; then
  TXHASH=$(cast send "${PRESS_TOKEN_ADDR}" "transfer(address,uint256)(bool)" "${TREASURY_ADDR}" "${AMOUNT_WEI}"     --rpc-url "${RPC_URL}" --private-key "${PAYER_PRIVATE_KEY}" | awk '/Transaction hash:/ {print $3}' | tail -n 1)
fi
if [[ -z "${TXHASH}" ]]; then
  echo "PAY_FEE_FAILED" >&2
  exit 4
fi
# confirm receipt
for i in $(seq 1 40); do
  if cast receipt "${TXHASH}" --rpc-url "${RPC_URL}" >/dev/null 2>&1; then
    echo "${TXHASH}"
    exit 0
  fi
  sleep 1
done
echo "RECEIPT_TIMEOUT|${TXHASH}" >&2
exit 5
