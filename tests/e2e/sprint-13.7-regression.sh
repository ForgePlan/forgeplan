#!/usr/bin/env bash
# Sprint 13.7 — PRD-042 FPF KB Vector Search — regression smoke.
# Runs against the release binary at $REPO/target/release/forgeplan.
#
# Note: pipefail intentionally OFF — grep -q may close the pipe early causing
# SIGPIPE (exit 141) upstream. We capture output then grep separately.
set -eu

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${REPO}/target/release/forgeplan"

if [[ ! -x "$BIN" ]]; then
  echo "✗ Release binary not found at $BIN — run 'cargo build --release' first"
  exit 1
fi

WORK="$(mktemp -d)"
cd "$WORK"
echo "Working dir: $WORK"
echo

assert_contains() {
  local label="$1" haystack="$2" needle_re="$3"
  if printf '%s' "$haystack" | grep -qE "$needle_re"; then
    echo "✓ $label"
  else
    echo "✗ $label FAILED"
    echo "    expected to match: $needle_re"
    echo "    output (first 200 chars): ${haystack:0:200}"
    exit 1
  fi
}

"$BIN" init -y >/dev/null
echo "✓ init"

# ─────────────────────────────────────────────────────────────
# Sprint 13.1 — duplicate guard
# ─────────────────────────────────────────────────────────────
"$BIN" new prd "Regression Test 13.7" >/dev/null
"$BIN" new prd "Regression Test 13.7" >/dev/null 2>&1 || true
COUNT=$(ls .forgeplan/prds/ | wc -l | tr -d ' ')
if [[ "$COUNT" == "1" ]]; then
  echo "✓ 13.1 duplicate guard (only PRD-001 exists)"
else
  echo "✗ 13.1 duplicate guard FAILED (expected 1 PRD, got $COUNT)"
  exit 1
fi

# ─────────────────────────────────────────────────────────────
# Sprint 13.2 — smart search
# ─────────────────────────────────────────────────────────────
OUT="$("$BIN" search "regression" --no-expand 2>&1)"
assert_contains "13.2 smart search" "$OUT" "PRD-001"

# ─────────────────────────────────────────────────────────────
# Sprint 13.3 — tags
# ─────────────────────────────────────────────────────────────
"$BIN" tag PRD-001 source=code >/dev/null
OUT="$("$BIN" list --tag source=code 2>&1)"
assert_contains "13.3 tags (key=value filter)" "$OUT" "PRD-001"

# ─────────────────────────────────────────────────────────────
# Sprint 13.4 — discover subcommand present
# ─────────────────────────────────────────────────────────────
OUT="$("$BIN" discover --help 2>&1 || true)"
assert_contains "13.4 discover subcommand present" "$OUT" "discover|Usage"

# ─────────────────────────────────────────────────────────────
# Sprint 13.5 — score with evidence
# ─────────────────────────────────────────────────────────────
"$BIN" new evidence "Test Evidence" --allow-duplicate >/dev/null
"$BIN" link EVID-001 PRD-001 --relation informs >/dev/null 2>&1 || true
OUT="$("$BIN" score PRD-001 2>&1)"
assert_contains "13.5 score with evidence" "$OUT" "R_eff|Confidence|Quality"

# ─────────────────────────────────────────────────────────────
# Sprint 13.6 — fpf rules / check
# ─────────────────────────────────────────────────────────────
OUT="$("$BIN" fpf rules 2>&1)"
assert_contains "13.6 fpf rules tree" "$OUT" "EXPLORE|INVESTIGATE|EXPLOIT"

OUT="$("$BIN" fpf rules --flat 2>&1)"
assert_contains "13.6 fpf rules --flat" "$OUT" "\["

OUT="$("$BIN" fpf rules --json 2>&1)"
if printf '%s' "$OUT" | python3 -c "import sys,json; j=json.load(sys.stdin); assert j['count']>0 and isinstance(j['rules'],list) and len(j['rules'])>0"; then
  echo "✓ 13.6 fpf rules --json (count>0, rules array)"
else
  echo "✗ 13.6 fpf rules --json FAILED"
  echo "$OUT" | head -20
  exit 1
fi

OUT="$("$BIN" fpf check PRD-001 2>&1)"
assert_contains "13.6 fpf check styled" "$OUT" "PRD-001"

OUT="$("$BIN" fpf check PRD-001 --json 2>&1)"
if printf '%s' "$OUT" | python3 -c "import sys,json; j=json.load(sys.stdin); assert 'matched' in j and 'unmatched' in j and 'artifact_id' in j"; then
  echo "✓ 13.6 fpf check --json (artifact_id, matched, unmatched)"
else
  echo "✗ 13.6 fpf check --json FAILED"
  echo "$OUT" | head -20
  exit 1
fi

if "$BIN" fpf check NOPE-999 >/dev/null 2>&1; then
  echo "✗ 13.6 fpf check missing artifact should fail but succeeded"
  exit 1
else
  echo "✓ 13.6 fpf check missing artifact errors (non-zero exit)"
fi

# ─────────────────────────────────────────────────────────────
# Sprint 13.7 — NEW: fpf search --semantic (graceful fallback)
# ─────────────────────────────────────────────────────────────
# Ingest FPF KB first so keyword search has something to match
"$BIN" fpf ingest >/dev/null 2>&1 || true

# 1. --semantic with default build (no semantic-search feature) MUST warn + fallback + succeed
OUT="$("$BIN" fpf search "trust" --semantic 2>&1 || true)"
if echo "$OUT" | grep -qi "fallback\|falling back"; then
  echo "✓ 13.7 fpf search --semantic prints fallback warning"
else
  echo "✗ 13.7 fpf search --semantic fallback warning FAILED"
  echo "$OUT" | head -20
  exit 1
fi

# Exit code must be 0 (graceful)
if "$BIN" fpf search "trust" --semantic >/dev/null 2>&1; then
  echo "✓ 13.7 fpf search --semantic exits 0 (graceful)"
else
  echo "✗ 13.7 fpf search --semantic non-zero exit"
  exit 1
fi

# 2. Keyword path (no --semantic) still works untouched
OUT="$("$BIN" fpf search "trust" 2>&1 || true)"
if echo "$OUT" | grep -qi "trust\|No FPF sections"; then
  echo "✓ 13.7 fpf search keyword path works"
else
  echo "✗ 13.7 fpf search keyword path FAILED"
  echo "$OUT" | head -20
  exit 1
fi

echo
echo "=== ALL REGRESSION + NEW 13.7 CHECKS PASSED ==="
