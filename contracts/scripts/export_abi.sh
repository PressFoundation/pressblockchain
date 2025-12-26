#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CONTRACTS_DIR="${ROOT_DIR}/contracts"
STATE_DIR="${ROOT_DIR}/state"
ABI_DIR="${STATE_DIR}/abi"

mkdir -p "${ABI_DIR}"

echo "[abi] Building contracts and exporting ABI to ${ABI_DIR}"

docker run --rm -v "${CONTRACTS_DIR}:/contracts" -w /contracts ghcr.io/foundry-rs/foundry:latest \
  forge build

# Copy ABIs (Foundry output is out/<File>.sol/<Contract>.json)
find "${CONTRACTS_DIR}/out" -type f -name "*.json" | while read -r f; do
  # Only keep the ABI fragment to reduce size; still compatible
  name="$(basename "$f")"
  if command -v jq >/dev/null 2>&1; then
    jq '{contractName:.contractName, abi:.abi}' "$f" > "${ABI_DIR}/${name}";
  else
    cp "$f" "${ABI_DIR}/${name}";
  fi
done

# Build address registry from state txt files
python3 - << 'PY'
import json, glob, os
state_dir=os.path.join(os.environ.get("STATE_DIR",""),"")  # unused
root=os.path.abspath(os.path.join(os.path.dirname(__file__),"..",".."))
sd=os.path.join(root,"state")
out={}
for p in glob.glob(os.path.join(sd,"*_address.txt")):
    key=os.path.basename(p).replace("_address.txt","")
    with open(p) as f:
        out[key]=f.readline().strip()
with open(os.path.join(sd,"contract_addresses.json"),"w") as f:
    json.dump(out,f,indent=2)
print("[abi] Wrote", os.path.join(sd,"contract_addresses.json"), "keys=", len(out))
PY

echo "[abi] Done."
