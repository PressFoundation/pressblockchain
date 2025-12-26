#!/usr/bin/env bash
set -euo pipefail
# Generates a new EVM wallet using cast (Foundry).
# Output format: address|private_key
ADDR_LINE=$(cast wallet new | tr -d '\r')
# expected lines contain "Address:" and "Private key:"
ADDR=$(echo "$ADDR_LINE" | awk '/Address:/ {print $2}' | tail -n 1)
PRIV=$(echo "$ADDR_LINE" | awk '/Private key:/ {print $3}' | tail -n 1)
if [[ -z "${ADDR}" || -z "${PRIV}" ]]; then
  echo "FAILED_TO_PARSE_WALLET" >&2
  echo "$ADDR_LINE" >&2
  exit 3
fi
echo "${ADDR}|${PRIV}"
