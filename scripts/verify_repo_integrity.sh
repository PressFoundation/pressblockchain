#!/usr/bin/env bash
set -euo pipefail
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST="$ROOT_DIR/REPO_MANIFEST.json"
python3 - <<'PY'
import json, hashlib, os, sys
root=os.path.abspath(os.path.join(os.path.dirname(__file__),".."))
m=json.load(open(os.path.join(root,"REPO_MANIFEST.json")))
bad=0
for f in m["files"]:
    p=os.path.join(root,f["path"])
    if not os.path.exists(p):
        print("MISSING", f["path"]); bad+=1; continue
    b=open(p,"rb").read()
    h=hashlib.sha256(b).hexdigest()
    if h!=f["sha256"] or len(b)!=f["bytes"]:
        print("MISMATCH", f["path"], "expected", f["sha256"], f["bytes"], "got", h, len(b)); bad+=1
print("OK" if bad==0 else f"FAILED: {bad} issues")
sys.exit(0 if bad==0 else 1)
PY
