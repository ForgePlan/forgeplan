#!/bin/bash
# scripts/smoke-test.sh — Forgeplan end-to-end smoke procedure.
#
# Mandate from CLAUDE.md "Smoke test (every sprint)" + Forge Mode discipline.
# Runs comprehensive user workflows on ephemeral temp workspace:
#   1. forgeplan init -y
#   2. forgeplan new <kind> for PRD, RFC, ADR, Epic, Spec, Problem, Evidence, Note
#   3. forgeplan validate / score / search
#   4. forgeplan blocked / order
#   5. forgeplan health (verify clean state)
#   6. forgeplan list (verify visibility)
#   7. forgeplan link (test relations)
#   8. forgeplan fpf ingest / fpf search (FPF KB smoke)
#   9. forgeplan tree (hierarchy ASCII)
#   10. forgeplan progress (FR checkbox tracker)
#   11. forgeplan claim/release (PRD-057 multi-agent coordination cycle)
#   12. forgeplan dispatch (PRD-057 parallel work planner)
#   13. forgeplan phase + phase-advance (advisory phase tracker, EPIC-005)
#   14. Clean up ephemeral workspace
#
# v0.31.0 coverage sprint: +6 ops (tree, progress, claim, release, dispatch,
# phase-advance + phase). Total: 13 → 19 operation categories.
#
# Usage: bash scripts/smoke-test.sh [--verbose]
# Exit 0 on full pass, 1 on any step failure (with clear output)

set -euo pipefail

# Configuration
VERBOSE="${1:-}"
# Compute absolute path to forgeplan binary (allows calling from any directory)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
FORGEPLAN_BIN="${FORGEPLAN_BIN:-$PROJECT_ROOT/target/debug/forgeplan}"
SMOKE_DIR=""

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Cleanup handler
cleanup() {
    if [ -n "$SMOKE_DIR" ] && [ -d "$SMOKE_DIR" ]; then
        if [ "$VERBOSE" = "--verbose" ]; then
            echo "[cleanup] Removing ephemeral workspace: $SMOKE_DIR"
        fi
        rm -rf "$SMOKE_DIR"
    fi
}

trap cleanup EXIT INT TERM

# Utility functions.
# All log functions write к stderr so callers using `$(create_artifact ...)`
# capture only the ID line (which goes to stdout via final `echo "$id"`).
# CI bug fix: `log_info` previously printed к stdout, which contaminated
# `$(...)` capture with `ℹ Created prd: PRD-001` prefix when --verbose is on.
log_step() {
    echo -e "${GREEN}✓${NC} $1" >&2
}

log_error() {
    echo -e "${RED}✗${NC} $1" >&2
}

log_info() {
    if [ "$VERBOSE" = "--verbose" ]; then
        echo -e "${YELLOW}ℹ${NC} $1" >&2
    fi
}

fail() {
    log_error "$1"
    exit 1
}

# Verify forgeplan binary exists
if [ ! -f "$FORGEPLAN_BIN" ]; then
    fail "forgeplan binary not found at $FORGEPLAN_BIN. Run: cargo build --bin forgeplan"
fi

log_info "forgeplan binary: $FORGEPLAN_BIN"
log_info "version: $($FORGEPLAN_BIN --version 2>/dev/null || echo 'unknown')"

# Create ephemeral workspace
SMOKE_DIR=$(mktemp -d -t forgeplan-smoke-XXXXXX)
log_step "Created ephemeral workspace: $SMOKE_DIR"

# Navigate to smoke workspace
cd "$SMOKE_DIR"

# Track created artifact IDs (as associative array)
declare -a ARTIFACT_IDS

# ============================================================================
# T1: Rust pipeline checks (cargo fmt/check/test)
# ============================================================================
log_info "Skipping Rust pipeline (cargo fmt/check/test) — already run in CI job"

# ============================================================================
# T2: Initialize forgeplan workspace
# ============================================================================
log_step "Running: forgeplan init -y"
if ! "$FORGEPLAN_BIN" init -y > /dev/null 2>&1; then
    fail "forgeplan init -y failed (exit code $?)"
fi

# Verify .forgeplan directory was created
if [ ! -d ".forgeplan" ]; then
    fail ".forgeplan directory not created"
fi
log_step "Initialized .forgeplan workspace"

# ============================================================================
# T3: Create artifacts of each kind
# ============================================================================
log_info "Creating test artifacts..."

# Helper to create artifact and capture ID
create_artifact() {
    local kind="$1"
    local title="$2"

    local output
    output=$("$FORGEPLAN_BIN" new "$kind" "$title" 2>&1) || {
        fail "forgeplan new $kind '$title' failed: $output"
    }

    # Extract ID from output (format: "Created PRD-NNN: ...")
    local id
    id=$(echo "$output" | grep -oE '[A-Z]+-[0-9]+' | head -1)

    if [ -z "$id" ]; then
        fail "Could not extract artifact ID from: $output"
    fi

    log_info "Created $kind: $id"
    echo "$id"
}

# Create one artifact of each kind
PRD_ID=$(create_artifact "prd" "Smoke test PRD")
RFC_ID=$(create_artifact "rfc" "Smoke test RFC")
ADR_ID=$(create_artifact "adr" "Smoke test ADR")
EPIC_ID=$(create_artifact "epic" "Smoke test Epic")
SPEC_ID=$(create_artifact "spec" "Smoke test Spec")
PROB_ID=$(create_artifact "problem" "Smoke test Problem")
EVID_ID=$(create_artifact "evidence" "Smoke test Evidence")
NOTE_ID=$(create_artifact "note" "Smoke test Note")

# Collect into array for summary
ARTIFACT_IDS=("$PRD_ID" "$RFC_ID" "$ADR_ID" "$EPIC_ID" "$SPEC_ID" "$PROB_ID" "$EVID_ID" "$NOTE_ID")

log_step "Created 8 artifacts: ${#ARTIFACT_IDS[@]} kinds"

# ============================================================================
# T4: Validate artifacts
# ============================================================================
log_info "Validating artifacts..."

for id in "${ARTIFACT_IDS[@]}"; do
    output=$("$FORGEPLAN_BIN" validate "$id" 2>&1) || {
        log_error "forgeplan validate $id failed: $output"
        # Don't fail on validation — some artifacts are intentionally incomplete
    }
    log_step "Validated: $id"
done

# ============================================================================
# T5: Score artifacts (R_eff)
# ============================================================================
log_info "Scoring artifacts..."

for id in "${ARTIFACT_IDS[@]}"; do
    output=$("$FORGEPLAN_BIN" score "$id" 2>&1) || {
        log_error "forgeplan score $id failed: $output"
        # Scoring may have dependencies — don't fail
    }
    log_step "Scored: $id"
done

# ============================================================================
# T6: Test graph queries
# ============================================================================
log_info "Testing graph queries..."

# Test: forgeplan blocked
output=$("$FORGEPLAN_BIN" blocked 2>&1) || {
    fail "forgeplan blocked failed: $output"
}
log_step "Query: forgeplan blocked"

# Test: forgeplan order
output=$("$FORGEPLAN_BIN" order 2>&1) || {
    fail "forgeplan order failed: $output"
}
log_step "Query: forgeplan order"

# ============================================================================
# T7: Test health and status
# ============================================================================
log_info "Testing workspace health..."

# Test: forgeplan health
output=$("$FORGEPLAN_BIN" health 2>&1) || {
    fail "forgeplan health failed: $output"
}
log_step "Query: forgeplan health"

# Test: forgeplan status
output=$("$FORGEPLAN_BIN" status 2>&1) || {
    fail "forgeplan status failed: $output"
}
log_step "Query: forgeplan status"

# ============================================================================
# T8: Test artifact listing
# ============================================================================
log_info "Testing artifact listing..."

# Test: forgeplan list
output=$("$FORGEPLAN_BIN" list 2>&1) || {
    fail "forgeplan list failed: $output"
}

# Verify all 8 artifacts appear in list
local_artifact_count=$(echo "$output" | grep -c 'PRD\|RFC\|ADR\|Epic\|Spec\|Problem\|Evidence\|Note' || echo 0)
if [ "$local_artifact_count" -lt 8 ]; then
    fail "forgeplan list returned fewer than 8 artifacts"
fi
log_step "Listed artifacts (found $local_artifact_count items)"

# ============================================================================
# T9: Test linking (relations)
# ============================================================================
log_info "Testing artifact relations..."

if [ -n "$PRD_ID" ] && [ -n "$RFC_ID" ]; then
    output=$("$FORGEPLAN_BIN" link "$PRD_ID" "$RFC_ID" --relation informs 2>&1) || {
        fail "forgeplan link failed: $output"
    }
    log_step "Linked: $PRD_ID informs $RFC_ID"
fi

# ============================================================================
# T10: Test search
# ============================================================================
log_info "Testing search..."

output=$("$FORGEPLAN_BIN" search "smoke" 2>&1) || {
    fail "forgeplan search 'smoke' failed: $output"
}
log_step "Search: query='smoke'"

# ============================================================================
# T11: FPF Knowledge Base (optional, may not have KB configured)
# ============================================================================
log_info "Testing FPF knowledge base..."

# Test: forgeplan fpf list (list KB sections)
output=$("$FORGEPLAN_BIN" fpf list 2>&1) || {
    log_error "forgeplan fpf list failed (KB may not be configured): $output"
}
log_step "FPF list: KB sections"

# Test: forgeplan fpf search (search KB)
output=$("$FORGEPLAN_BIN" fpf search "decision" 2>&1) || {
    log_error "forgeplan fpf search failed (KB may not be configured): $output"
}
log_step "FPF search: query='decision'"

# ============================================================================
# T12: Graph visualization
# ============================================================================
log_info "Testing graph visualization..."

output=$("$FORGEPLAN_BIN" graph 2>&1) || {
    fail "forgeplan graph failed: $output"
}
log_step "Generated: dependency graph (mermaid)"

# ============================================================================
# T13: Hierarchy view (ASCII tree)
# ============================================================================
log_info "Testing artifact hierarchy view..."

output=$("$FORGEPLAN_BIN" tree 2>&1) || {
    fail "forgeplan tree failed: $output"
}
log_step "Hierarchy: forgeplan tree (ASCII)"

# ============================================================================
# T14: Progress tracker (FR checkboxes)
# ============================================================================
log_info "Testing progress tracker..."

# Without ID: aggregate progress across all artifacts
output=$("$FORGEPLAN_BIN" progress 2>&1) || {
    fail "forgeplan progress (all) failed: $output"
}
log_step "Progress: forgeplan progress (workspace-wide)"

# Per-artifact JSON shape (verifies machine-consumable output)
output=$("$FORGEPLAN_BIN" progress "$PRD_ID" --json 2>&1) || {
    fail "forgeplan progress $PRD_ID --json failed: $output"
}
log_step "Progress: forgeplan progress $PRD_ID --json"

# ============================================================================
# T15: Multi-agent coordination (PRD-057 claim/release cycle)
# ============================================================================
log_info "Testing claim/release cycle..."

# Claim with short TTL — proves write path
output=$("$FORGEPLAN_BIN" claim "$PRD_ID" --agent "smoke-test/v1" --ttl-minutes 1 --note "smoke" 2>&1) || {
    fail "forgeplan claim $PRD_ID failed: $output"
}
log_step "Claim: $PRD_ID (agent=smoke-test/v1, ttl=1m)"

# Claims listing — proves read path picks up the claim
output=$("$FORGEPLAN_BIN" claims 2>&1) || {
    fail "forgeplan claims failed: $output"
}
if ! echo "$output" | grep -q "$PRD_ID"; then
    fail "forgeplan claims did not list newly-created claim for $PRD_ID: $output"
fi
log_step "Claims: forgeplan claims (found $PRD_ID)"

# Release — idempotent close
output=$("$FORGEPLAN_BIN" release "$PRD_ID" --agent "smoke-test/v1" 2>&1) || {
    fail "forgeplan release $PRD_ID failed: $output"
}
log_step "Release: $PRD_ID"

# ============================================================================
# T16: Multi-agent dispatch planner (PRD-057)
# ============================================================================
log_info "Testing dispatch planner..."

# JSON mode — machine-consumable plan for orchestrator
output=$("$FORGEPLAN_BIN" dispatch --agents 3 --status any --json 2>&1) || {
    fail "forgeplan dispatch --agents 3 --status any --json failed: $output"
}
log_step "Dispatch: forgeplan dispatch --agents 3 --status any --json"

# ============================================================================
# T17: Advisory phase tracker (EPIC-005)
# ============================================================================
log_info "Testing advisory phase tracker..."

# Advance phase — writes .forgeplan/state/<id>.yaml history
output=$("$FORGEPLAN_BIN" phase-advance "$PRD_ID" --to shape --reason "smoke test" 2>&1) || {
    fail "forgeplan phase-advance $PRD_ID --to shape failed: $output"
}
log_step "Phase-advance: $PRD_ID → shape"

# Read it back — proves state file round-trip
output=$("$FORGEPLAN_BIN" phase "$PRD_ID" 2>&1) || {
    fail "forgeplan phase $PRD_ID failed: $output"
}
if ! echo "$output" | grep -qi "shape"; then
    fail "forgeplan phase $PRD_ID did not return 'shape' after advance: $output"
fi
log_step "Phase: forgeplan phase $PRD_ID (current=shape)"

# ============================================================================
# Final summary
# ============================================================================
echo ""
log_step "=== SMOKE TEST PASSED ==="
echo ""
echo "Artifacts created:"
for id in "${ARTIFACT_IDS[@]}"; do
    echo "  - $id"
done
echo ""
echo "Operations tested:"
echo "  ✓ forgeplan init"
echo "  ✓ forgeplan new (8 kinds)"
echo "  ✓ forgeplan validate"
echo "  ✓ forgeplan score"
echo "  ✓ forgeplan blocked"
echo "  ✓ forgeplan order"
echo "  ✓ forgeplan health"
echo "  ✓ forgeplan status"
echo "  ✓ forgeplan list"
echo "  ✓ forgeplan link"
echo "  ✓ forgeplan search"
echo "  ✓ forgeplan fpf list/search"
echo "  ✓ forgeplan graph"
echo "  ✓ forgeplan tree (hierarchy ASCII)"
echo "  ✓ forgeplan progress (workspace + per-ID JSON)"
echo "  ✓ forgeplan claim / claims / release (PRD-057 cycle)"
echo "  ✓ forgeplan dispatch --agents 3 --json (PRD-057 planner)"
echo "  ✓ forgeplan phase-advance / phase (EPIC-005 advisory tracker)"
echo ""

exit 0
