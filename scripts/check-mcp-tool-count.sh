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

# Source-of-truth count: each `#[tool(...)]` macro attribute registers one
# MCP tool with the rmcp runtime. We anchor on the macro itself rather than
# the function declaration, because inline `#[cfg(test)]` test functions
# share the `forgeplan_` prefix (e.g. `forgeplan_get_accepts_slug_or_display_id`)
# but are NOT decorated с `#[tool(...)]` — counting `async fn forgeplan_*`
# inflated the count by the number of inline tests (issue #306). The
# `^[[:space:]]*` anchor ignores any indentation but still requires the
# attribute to start its own line.
ACTUAL=$(grep -cE '^[[:space:]]*#\[tool\(' "$SERVER_RS" 2>/dev/null || echo 0)

if [[ "$ACTUAL" -lt 30 ]]; then
    echo "FAIL: cannot find MCP tools в $SERVER_RS (found $ACTUAL — expected ≥30)" >&2
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
# / "76 CLI commands"). Word boundary anchored с trailing non-letter
# чтобы не matchать `tooltip`/`toolbox`/`toolkit` (audit C-3 finding).
#
# Separator between `MCP` and the noun is [[:space:]-]*, NOT whitespace-only:
# Russian writes the compound hyphenated (`73 MCP-инструмента`), which is the
# idiomatic form and therefore what the RU pages actually use. With a
# whitespace-only class this script scanned website/src, walked straight past
# `63 MCP-инструмента` in ru/docs/cli/serve.md, and printed "No drift" — a
# green gate certifying the very drift it exists to catch. English `MCP-tools`
# has the same exposure. See #421.
EXTRACT_RE='[0-9]+[[:space:]]*(MCP[[:space:]-]*tool[s]?([^a-zA-Z]|$)|tool[s]?([^a-zA-Z]|$)|MCP[[:space:]-]*инструмент|инструмент)'

# --self-test: exercise the extraction regex against both languages and both
# separators. Runs offline, touches no repo file. The blind spot in #421
# survived because nothing ever asserted the pattern itself.
if [[ "${SELF_TEST:-0}" == "1" ]]; then
    st_fail=0
    st_expect() { # <should-match 0|1> <sample>
        local want="$1" sample="$2" got
        got=$(printf '%s' "$sample" | grep -coE "$EXTRACT_RE")
        if [[ "$got" -ne "$want" ]]; then
            echo "  SELF-TEST FAIL: expected match=$want got=$got for: $sample" >&2
            st_fail=1
        else
            echo "  ok (match=$got): $sample"
        fi
    }
    echo "Self-test: EXTRACT_RE"
    st_expect 1 '73 MCP tools for agents'
    st_expect 1 '73 MCP-tools for agents'
    st_expect 1 '73 tools for agents'
    st_expect 1 '73 MCP инструмента для агентов'
    st_expect 1 '73 MCP-инструмента, сопоставленных с операциями'
    st_expect 1 '73 инструмента для агентов'
    # Must NOT match — the audit C-3 false positives.
    st_expect 0 'a tooltip and a toolbox'
    st_expect 0 '76 CLI commands'
    if [[ "$st_fail" -ne 0 ]]; then
        echo "Self-test FAILED" >&2
        exit 1
    fi
    echo "Self-test OK"
    exit 0
fi

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
        | grep -v 'TODO\.md.*Previous: v0\.' \
        | grep -v 'mcp-count-drift: ignore' \
        | while read -r line; do
            # NOTE: DRIFT_FOUND is NOT assigned inside this while-pipe
            # (subshell scoping: variable changes don't propagate to
            # parent). We use a tempfile ($DRIFT_OUTPUT) и check
            # `[[ -s "$DRIFT_OUTPUT" ]]` после loop exit. Don't refactor
            # this в `DRIFT_FOUND=1` inside the loop — silent breakage.
            # Audit C-5 lessons learned 2026-05-04.
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
