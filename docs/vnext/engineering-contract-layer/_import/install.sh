#!/usr/bin/env bash
set -euo pipefail
PACK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_DIR="${1:-$(pwd)}"
if [[ ! -d "$REPO_DIR/.git" ]]; then echo "ERROR: $REPO_DIR is not a Git repository root" >&2; exit 1; fi
SRC_DOCS="$PACK_DIR/payload/docs/vnext/engineering-contract-layer"
DST_DOCS="$REPO_DIR/docs/vnext/engineering-contract-layer"
SRC_PR="$PACK_DIR/payload/.github/PULL_REQUEST_TEMPLATE/forgeplan-vnext.md"
DST_PR="$REPO_DIR/.github/PULL_REQUEST_TEMPLATE/forgeplan-vnext.md"
mkdir -p "$DST_DOCS" "$(dirname "$DST_PR")"
# Refuse destructive overwrite; identical files are fine.
conflicts=0
while IFS= read -r -d '' f; do
  rel="${f#$SRC_DOCS/}"; dst="$DST_DOCS/$rel"
  if [[ -f "$dst" ]] && ! cmp -s "$f" "$dst"; then echo "CONFLICT: $dst" >&2; conflicts=1; fi
done < <(find "$SRC_DOCS" -type f -print0)
if [[ -f "$DST_PR" ]] && ! cmp -s "$SRC_PR" "$DST_PR"; then echo "CONFLICT: $DST_PR" >&2; conflicts=1; fi
if [[ "$conflicts" -ne 0 ]]; then
  echo "Import stopped. Reconcile conflicts explicitly, then rerun." >&2; exit 2
fi
rsync -a "$SRC_DOCS/" "$DST_DOCS/"
cp "$SRC_PR" "$DST_PR"
echo "Installed docs to $DST_DOCS"
echo "Installed PR template to $DST_PR"
echo "Next: python3 $DST_DOCS/scripts/validate_pack.py"
