#!/usr/bin/env bash
set -euo pipefail
FILE="${1:-state/feature_flags.json}"
KEY="${2:-}"
VAL="${3:-}"

if [[ -z "$KEY" || -z "$VAL" ]]; then
  echo "Usage: bash ops/scripts/toggle_feature.sh <feature_flags.json> <feature_key> <true|false>"
  exit 1
fi

python3 - <<'PY'
import json,sys
file=sys.argv[1]; key=sys.argv[2]; val=sys.argv[3].lower()=='true'
with open(file,'r') as f: data=json.load(f)
data.setdefault('features',{}).setdefault(key,{'enabled':True})
data['features'][key]['enabled']=val
with open(file,'w') as f: json.dump(data,f,indent=2)
print(f"[ok] set {key} enabled={val} in {file}")
PY
