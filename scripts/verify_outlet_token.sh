#!/usr/bin/env bash
set -euo pipefail

# Required env:
# RPC_URL, OWNER_PRIVATE_KEY, TOKEN_ADDR, TREASURY_ADDR, TEST_AMOUNT_WEI

if [[ -z "${RPC_URL:-}" || -z "${OWNER_PRIVATE_KEY:-}" || -z "${TOKEN_ADDR:-}" || -z "${TREASURY_ADDR:-}" || -z "${TEST_AMOUNT_WEI:-}" ]]; then
  echo "Missing required env vars" >&2
  exit 2
fi

# Verify basic metadata via eth_call
SYMBOL=$(cast call "${TOKEN_ADDR}" "symbol()(string)" --rpc-url "${RPC_URL}" || true)
DECIMALS=$(cast call "${TOKEN_ADDR}" "decimals()(uint8)" --rpc-url "${RPC_URL}" || true)
SUPPLY=$(cast call "${TOKEN_ADDR}" "totalSupply()(uint256)" --rpc-url "${RPC_URL}" || true)

if [[ -z "${SYMBOL}" || -z "${DECIMALS}" || -z "${SUPPLY}" ]]; then
  echo "METADATA_CALL_FAILED" >&2
  exit 3
fi

# Perform test transfer from owner to treasury
TXHASH=$(cast send "${TOKEN_ADDR}" "transfer(address,uint256)(bool)" "${TREASURY_ADDR}" "${TEST_AMOUNT_WEI}"   --rpc-url "${RPC_URL}" --private-key "${OWNER_PRIVATE_KEY}" | awk '/transactionHash/ {print $2}' | tail -n 1)

if [[ -z "${TXHASH}" ]]; then
  # cast output differs per version; fallback parse "Transaction hash:"
  TXHASH=$(cast send "${TOKEN_ADDR}" "transfer(address,uint256)(bool)" "${TREASURY_ADDR}" "${TEST_AMOUNT_WEI}"     --rpc-url "${RPC_URL}" --private-key "${OWNER_PRIVATE_KEY}" | awk '/Transaction hash:/ {print $3}' | tail -n 1)
fi

if [[ -z "${TXHASH}" ]]; then
  echo "TEST_TX_FAILED" >&2
  exit 4
fi

# Wait for receipt (cast receipt returns non-zero if not found)
for i in $(seq 1 40); do
  if cast receipt "${TXHASH}" --rpc-url "${RPC_URL}" >/dev/null 2>&1; then
    echo "${SYMBOL}|${DECIMALS}|${SUPPLY}|${TXHASH}"
    exit 0
  fi
  sleep 1
done

echo "RECEIPT_TIMEOUT|${TXHASH}" >&2
exit 5
