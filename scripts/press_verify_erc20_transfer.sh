#!/usr/bin/env bash
set -euo pipefail
# Verify an ERC20 Transfer(to,amount) event in a tx receipt.
# Usage: press_verify_erc20_transfer.sh <rpc> <txhash> <token> <to> <min_amount_wei>
RPC="${1:?rpc}"
TX="${2:?txhash}"
TOKEN="${3:?token}"
TO="${4:?to}"
MIN_AMT="${5:?min_amount_wei}"

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Get receipt JSON
REC_JSON=$("${DIR}/_foundry.sh" cast receipt --rpc-url "${RPC}" --json "${TX}")

python3 - <<PY
import json, sys
rec=json.loads('''${REC_JSON}''')
token='${TOKEN}'.lower()
to='${TO}'.lower()
min_amt=int('${MIN_AMT}')
logs=rec.get('logs',[]) or []
# Transfer(address,address,uint256) topic
TRANSFER_TOPIC='0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef'
ok=False
best=0
for lg in logs:
    addr=(lg.get('address') or '').lower()
    if addr!=token: 
        continue
    topics=lg.get('topics') or []
    if len(topics)<3: 
        continue
    if (topics[0] or '').lower()!=TRANSFER_TOPIC:
        continue
    # topics[2] is 'to' address padded
    to_topic=(topics[2] or '').lower()
    if not to_topic.endswith(to.replace('0x','')):
        continue
    data=lg.get('data') or '0x0'
    amt=int(data,16) if data.startswith('0x') else int(data)
    best=max(best,amt)
    if amt>=min_amt:
        ok=True
        break
print(json.dumps({"ok":ok,"bestAmount":best,"tx":rec.get("transactionHash")})) 
PY
