#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
python3 "$ROOT/payload/docs/vnext/engineering-contract-layer/scripts/validate_pack.py"
python3 -m json.tool "$ROOT/MANIFEST.json" >/dev/null
find "$ROOT" -name '__pycache__' -o -name '*.pyc' | grep -q . && { echo 'ERROR: cache files found'; exit 1; } || true
echo "Package validation OK"
