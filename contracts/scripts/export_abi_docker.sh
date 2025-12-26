#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CONTRACTS_DIR="${ROOT_DIR}/contracts"
STATE_DIR="${ROOT_DIR}/state"
ABI_DIR="${STATE_DIR}/abi"
mkdir -p "${ABI_DIR}"

echo "[abi] Building + exporting via containers (no host deps)"
docker run --rm -v "${CONTRACTS_DIR}:/contracts" -w /contracts ghcr.io/foundry-rs/foundry:latest forge build

# copy full artifacts to state/abi
docker run --rm -v "${CONTRACTS_DIR}:/contracts" -v "${ABI_DIR}:/abi" alpine:3.20 sh -lc 'cp -r /contracts/out/* /abi/ || true'

# address registry from state txt
docker run --rm -v "${STATE_DIR}:/state" python:3.11-slim python - << "PY"
import json, glob, os
out={}
for p in glob.glob("/state/*_address.txt"):
    key=os.path.basename(p).replace("_address.txt","")
    with open(p) as f:
        out[key]=f.readline().strip()
with open("/state/contract_addresses.json","w") as f:
    json.dump(out,f,indent=2)
print("wrote contract_addresses.json keys",len(out))
PY

echo "[abi] Done."
