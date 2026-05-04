#!/usr/bin/env bash
#
# check-mcp-tool-count.sh — Drift detector для MCP tool count.
#
# Считает actual MCP tools в src + находит все documentation claims
# что hardcode количество tools. Если расходятся — выводит каждую stale
# location + exit 1 (CI gate) или warning-only с --warn-only.
#
# Background: PROB-050 v0.28.0 release audit обнаружил 18 различных
# locations с stale tool counts (28 / 37 / 45 / 47), при actual count 63.
# OpenAI агент external review нашёл это раньше, чем internal sweep.
# Этот script предотвращает повторение.
#
# Использование:
#   scripts/check-mcp-tool-count.sh           # exit 1 при drift
#   scripts/check-mcp-tool-count.sh --warn    # warn-only (для local dev)
#   scripts/check-mcp-tool-count.sh --fix     # auto-fix simple cases (TBD)
#
# CI integration:
#   .github/workflows/health-gate.yml — добавить step после `cargo test`.
#
# Pre-commit hook (optional):
#   .git/hooks/pre-commit — call с --warn для удобства разработчика.
#

set -uo pipefail
# NOTE: no `set -e` — grep returning non-zero (no matches) is normal в этом
# pipeline и не должен kill'ать скрипт.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVER_RS="$REPO_ROOT/crates/forgeplan-mcp/src/server.rs"

WARN_ONLY=0
for arg in "$@"; do
    case "$arg" in
        --warn) WARN_ONLY=1 ;;
        --warn-only) WARN_ONLY=1 ;;
        -h|--help)
            sed -n '2,30p' "${BASH_SOURCE[0]}" | sed 's/^# *//'
            exit 0
            ;;
    esac
done

# Source-of-truth count: each `async fn forgeplan_*(` function decorated
# with `#[tool(...)]` macro is one MCP tool. The macro spans multiple
# lines so we anchor on the function declaration itself.
ACTUAL=$(grep -cE 'async fn forgeplan_' "$SERVER_RS" 2>/dev/null || echo 0)

if [[ "$ACTUAL" -lt 10 ]]; then
    echo "FAIL: cannot find MCP tools в $SERVER_RS (found $ACTUAL — expected ≥10)" >&2
    echo "      Has rmcp tool macro contract changed? Update this script." >&2
    exit 2
fi

echo "Actual MCP tool count (src): $ACTUAL"
echo ""

# Search docs/landing/CLAUDE/README/marketplace for hardcoded counts.
# We allow 0.27.0-era CHANGELOG entries to keep their historical numbers.
SEARCH_PATHS=(
    "$REPO_ROOT/CLAUDE.md"
    "$REPO_ROOT/README.md"
    "$REPO_ROOT/TODO.md"
    "$REPO_ROOT/website/src"
    "$REPO_ROOT/docs"
)

# Patterns that look like "<NUM> [MCP|tool|инструмент]" — extract ONLY
# the number that immediately precedes a tool/MCP/инструмент keyword,
# not any other number on the same line (avoid false-positive "1940 tests"
# / "58 CLI commands").
EXTRACT_RE='[0-9]+[[:space:]]*(MCP[[:space:]]*tool|tool[s]?|MCP[[:space:]]*инструмент|инструмент)'

DRIFT_FOUND=0
DRIFT_OUTPUT=$(mktemp)
trap 'rm -f "$DRIFT_OUTPUT"' EXIT

for path in "${SEARCH_PATHS[@]}"; do
    if [[ ! -e "$path" ]]; then
        continue
    fi

    grep -rEn "$EXTRACT_RE" "$path" \
        --include="*.md" \
        --include="*.tsx" \
        --include="*.astro" \
        --include="*.mdx" \
        2>/dev/null \
        | grep -v changelog \
        | grep -v node_modules \
        | grep -v 'website/dist' \
        | grep -v 'BROWNFIELD-ORCHESTRATOR-HANDOFF' \
        | grep -v 'TODO\.md.*Previous: v0\.' \
        | grep -v 'mcp-count-drift: ignore' \
        | while read -r line; do
            # Extract ONLY the number directly preceding tool/MCP/инструмент.
            # `grep -oE` returns the keyword match; we need the number too.
            # Capture: re-grep with full pattern and extract the leading [0-9]+.
            matches=$(echo "$line" | grep -oE "$EXTRACT_RE" || true)
            if [[ -z "$matches" ]]; then continue; fi
            # Each match line starts with the number — extract it.
            while IFS= read -r m; do
                num=$(echo "$m" | grep -oE '^[0-9]+')
                if [[ -z "$num" ]]; then continue; fi
                # Skip "subset" counts (< 10) — these usually refer to N most-
                # frequent tools, не общему числу. False positives like
                # "6 tools cover 90%" or "3 tools used in H2 test".
                if [[ "$num" -lt 10 ]]; then continue; fi
                if [[ "$num" != "$ACTUAL" ]]; then
                    echo "  DRIFT: $line  (number=$num context=\"$m\")" >> "$DRIFT_OUTPUT"
                fi
            done <<< "$matches"
        done
done

if [[ -s "$DRIFT_OUTPUT" ]]; then
    DRIFT_LINES=$(wc -l < "$DRIFT_OUTPUT" | tr -d ' ')
    echo "Drift detected ($DRIFT_LINES lines):"
    cat "$DRIFT_OUTPUT"
    echo ""
    echo "Resolution: update each location to actual count ($ACTUAL) OR add a"
    echo "comment explaining why the historical number is preserved (e.g. CHANGELOG)."
    DRIFT_FOUND=1
fi

if [[ "$DRIFT_FOUND" -eq 0 ]]; then
    echo "✅ No drift — all docs аре consistent с src ($ACTUAL tools)."
    exit 0
fi

if [[ "$WARN_ONLY" -eq 1 ]]; then
    echo ""
    echo "⚠️  Warning-only mode — exit 0 despite drift."
    exit 0
fi

exit 1
