#!/usr/bin/env bash
#
# Exercise every CLI command the way an agent or a person would, on a real
# binary, against a real workspace — and report what actually happened.
#
# Why this exists
# ---------------
# `scripts/smoke-test.sh` covers 21 of 82 commands. The Rust suite invokes more,
# but "the string appears in a test" is not "the behaviour is verified" — the
# gap between those two is where PROB-093 lived: `search` was exercised in
# tests for years while nobody asked whether a freshly created artifact could
# actually be found.
#
# So this harness asserts on OUTPUT, not on exit status alone. A command that
# exits 0 and prints nothing useful is recorded as a failure, because from an
# agent's point of view that is what it is.
#
# Verdicts
# --------
#   PASS      ran and produced the expected observable result
#   FAIL      ran and did not — a defect, or an assertion that needs fixing
#   EXTERNAL  needs something this harness deliberately does not provide
#             (an LLM key, a 2.1 GB model, a network fetch, an installed skill).
#             NOT a pass. It means "unverified here", and the summary says so.
#   SKIP      structurally cannot run unattended (long-running servers/watchers)
#
# EXTERNAL and SKIP are printed separately from PASS on purpose. Folding them
# into a green total is how a harness starts lying.
#
# Usage
#   scripts/cli-surface-exercise.sh [--bin <path>] [--verbose] [--keep]
#
set -uo pipefail

BIN="${FORGEPLAN_BIN:-forgeplan}"
VERBOSE=0
KEEP=0

while [ $# -gt 0 ]; do
  case "$1" in
    --bin) BIN="$2"; shift 2 ;;
    --verbose) VERBOSE=1; shift ;;
    --keep) KEEP=1; shift ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

if ! command -v "$BIN" >/dev/null 2>&1 && [ ! -x "$BIN" ]; then
  echo "forgeplan binary not found: $BIN" >&2
  echo "pass --bin <path> or set FORGEPLAN_BIN" >&2
  exit 2
fi

# Resolve to an absolute path BEFORE the `cd` into the throwaway workspace.
# A relative `--bin target/debug/forgeplan` — which is exactly what CI passes —
# stops resolving the moment we change directory, and every command then fails
# with exit 127. The existence check above happens pre-`cd`, so it passes and
# the breakage looks like 66 broken commands rather than one broken path.
case "$BIN" in
  /*) ;;
  */*) BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")" ;;
  *)  ;;  # bare name on PATH — leave it for the shell to resolve
esac

WORK="$(mktemp -d "${TMPDIR:-/tmp}/fpl-surface.XXXXXX")"
LOG="$WORK/transcript.log"
RESULTS="$WORK/results.tsv"
: > "$RESULTS"

cleanup() {
  if [ "$KEEP" = "1" ]; then
    echo "workspace kept at: $WORK"
  else
    rm -rf "$WORK"
  fi
}
trap cleanup EXIT

pass_n=0; fail_n=0; ext_n=0; skip_n=0

record() { # verdict, command, note
  printf '%s\t%s\t%s\n' "$1" "$2" "$3" >> "$RESULTS"
  case "$1" in
    PASS) pass_n=$((pass_n+1)) ;;
    FAIL) fail_n=$((fail_n+1)) ;;
    EXTERNAL) ext_n=$((ext_n+1)) ;;
    SKIP) skip_n=$((skip_n+1)) ;;
  esac
  if [ "$VERBOSE" = "1" ] || [ "$1" = "FAIL" ]; then
    printf '  %-9s %-20s %s\n' "$1" "$2" "$3"
  fi
}

# Run a command, capture output+status. Never aborts the harness.
run() {
  local out status
  out="$("$BIN" "$@" 2>&1)"; status=$?
  printf '\n=== %s %s (exit %d) ===\n%s\n' "$BIN" "$*" "$status" "$out" >> "$LOG"
  LAST_OUT="$out"; LAST_STATUS=$status
  return 0
}

# assert: label, expected-substring. Uses LAST_OUT/LAST_STATUS.
expect() {
  local label="$1" needle="$2"
  if [ "$LAST_STATUS" -ne 0 ]; then
    record FAIL "$label" "exit $LAST_STATUS: $(printf '%s' "$LAST_OUT" | head -1 | cut -c1-90)"
  elif printf '%s' "$LAST_OUT" | grep -qF "$needle"; then
    record PASS "$label" ""
  else
    record FAIL "$label" "output lacks '$needle': $(printf '%s' "$LAST_OUT" | head -1 | cut -c1-70)"
  fi
}

# expect_external: a non-zero exit that names a missing prerequisite is an
# honest refusal, not a defect. A non-zero exit for any other reason is a FAIL.
expect_external() {
  local label="$1" marker="$2"
  if [ "$LAST_STATUS" -eq 0 ]; then
    record PASS "$label" ""
  elif printf '%s' "$LAST_OUT" | grep -qiE "$marker"; then
    record EXTERNAL "$label" "refused cleanly: $(printf '%s' "$LAST_OUT" | grep -iEm1 "$marker" | cut -c1-70)"
  else
    record FAIL "$label" "exit $LAST_STATUS: $(printf '%s' "$LAST_OUT" | head -1 | cut -c1-90)"
  fi
}

cd "$WORK"
git init -q . 2>/dev/null || true
git config user.email "harness@example.com" 2>/dev/null || true
git config user.name "surface harness" 2>/dev/null || true

echo "Exercising CLI surface with: $BIN"
"$BIN" --version 2>&1 | head -1
echo

# ---------------------------------------------------------------------------
# 1. Workspace lifecycle
# ---------------------------------------------------------------------------
run init -y;                    expect init "Initialized"
run status;                     expect status ""
run migrate;                    expect_external migrate "no migration|already|up to date"
run migrate-dry-run;            expect_external migrate-dry-run "collision|scanned|artifacts|no "
run migrate-secrets;            expect_external migrate-secrets "dry|no known|would|secrets"

# ---------------------------------------------------------------------------
# 2. Artifact CRUD  — the spine every other command depends on
# ---------------------------------------------------------------------------
run new prd "Payment retry policy";        expect new "PRD-001"
run new rfc "Retry backoff design";        expect "new rfc" "RFC-001"
run new adr "Use exponential backoff";     expect "new adr" "ADR-001"
run new evidence "Retry bench: p95 180ms"; expect "new evidence" "EVID-001"
run new problem "Retries storm the API";   expect "new problem" "PROB-001"
run new note "Ops asked for a cap";        expect "new note" "NOTE-001"
run new spec "Retry API contract";         expect "new spec" "SPEC-001"
run new epic "Resilience work";            expect "new epic" "EPIC-001"

run get PRD-001;                expect get "PRD-001"
run list;                       expect list "PRD-001"

cat > "$WORK/body.md" <<'BODY'
## Problem

Payment retries currently hammer the upstream API without a ceiling.

## Goals

- Cap retry attempts
- Preserve idempotency

## Non-Goals

- Changing the payment provider

## Functional Requirements

- FR-001: cap attempts at 5
- FR-002: exponential backoff with jitter

## Target Users

Backend engineers operating the payment path.

## Related Artifacts

RFC-001
BODY
run update PRD-001 --body "@$WORK/body.md"; expect update "Updated"
run validate PRD-001;                       expect validate "PASS"

# ---------------------------------------------------------------------------
# 3. Links, graph, ordering
# ---------------------------------------------------------------------------
run link EVID-001 PRD-001 --relation informs; expect link "Linked"
run link RFC-001 PRD-001 --relation based_on; expect "link based_on" "Linked"
run graph;                      expect graph "graph"
run tree;                       expect_external tree "PRD-001|EPIC-001|No "
run order;                      expect_external order "PRD-001|order|No "
run blocked;                    expect_external blocked "blocked|No |0 "
run context PRD-001;            expect context "PRD-001"

# ---------------------------------------------------------------------------
# 4. Lifecycle state machine
# ---------------------------------------------------------------------------
run review PRD-001;             expect review "PRD-001"
run score PRD-001;              expect score "R_eff"
run activate PRD-001;           expect activate "active"
run new prd "Second payment PRD"; expect "new prd 2" "PRD-002"
run activate PRD-002;           expect_external "activate PRD-002" "validation|MUST|error|evidence"
run supersede PRD-001 --by PRD-002; expect_external supersede "superseded|Superseded|cannot|must"
run undo-last;                  expect_external undo-last "restore|Restored|receipt|no "
run deprecate NOTE-001 --reason "harness exercise"; expect_external deprecate "Deprecated|draft|transition"
run restore NOTE-001;           expect_external restore "Restored|receipt|no "
run renew PRD-002 --reason "harness" --until 2027-01-01; expect_external renew "Renewed|stale|not stale"
run reopen PROB-001 --reason "harness exercise";         expect_external reopen "Reopened|draft|new"

# ---------------------------------------------------------------------------
# 5. Quality, scoring, dashboards — what an agent reads to decide what is next
# ---------------------------------------------------------------------------
run health;                     expect health ""
run blindspots;                 expect_external blindspots "blind|No |0 |spot"
run gaps;                       expect_external gaps "gap|No |0 "
run anomalies;                  expect_external anomalies "anomal|No |0 "
run decay;                      expect_external decay "decay|No |0 |R_eff"
run fgr PRD-002;                expect_external fgr "F-G-R|Formality|F=|score"
run drift;                      expect_external drift "drift|No |0 "
run coverage;                   expect_external coverage "coverage|No |0 |module"
run progress;                   expect_external progress "progress|%|No |0 "
run journal;                    expect_external journal "journal|PRD|No |0 "
run log;                        expect_external log "log|PRD|No |0 "
run stale;                      expect_external stale "stale|No |0 "

# ---------------------------------------------------------------------------
# 6. Routing and planning
# ---------------------------------------------------------------------------
run route "add a retry cap to the payment client"; expect route "Depth"
run calibrate PRD-002;          expect_external calibrate "Depth|Tactical|Standard|Deep|Critical"
run estimate PRD-002;           expect_external estimate "hour|estimate|point|FR"
run calibrate-estimate --help;  expect "calibrate-estimate --help" "actual"
run reason PRD-002;             expect_external reason "LLM|API key|provider|timed out|GEMINI|not configured"
run decompose PRD-002;          expect_external decompose "LLM|API key|provider|timed out|GEMINI|not configured"
run generate prd "A generated PRD about caching"; expect_external generate "LLM|API key|provider|timed out|GEMINI|not configured"

# ---------------------------------------------------------------------------
# 7. Search and knowledge — PROB-093 territory
# ---------------------------------------------------------------------------
run search "retry";             expect search "PRD"
run search "retry" --semantic;  expect_external "search --semantic" "model|semantic-search|No semantic|not available"
run embed;                      expect_external embed "model|semantic-search|Done:|not available"
run scan;                       expect_external scan "module|scan|No |0 "
run fpf status;                 expect_external "fpf status" "not found|no |ingest|empty|sections"
run fpf search "trust";         expect_external "fpf search" "not found|no match|ingest|sections"

# PROB-093 regression, on the real binary: a brand-new artifact must be
# findable, and a rewritten one must stop matching what it no longer says.
run new note "Sourdough hydration ratios";  expect "new note 2" "NOTE-002"
run search "sourdough";                     expect "search finds new artifact" "NOTE-002"

# ---------------------------------------------------------------------------
# 8. Multi-agent coordination — agents depend on these being honest
# ---------------------------------------------------------------------------
run claim PRD-002 --agent harness --ttl-minutes 5; expect claim "PRD-002"
run claims;                     expect claims "PRD-002"
run release PRD-002 --agent harness; expect release "Released"
run claims;                     expect_external "claims after release" "No |0 |empty|active"
run dispatch --agents 3;        expect_external dispatch "agent|bucket|plan|queue"
run session;                    expect_external session "phase|session|No |unknown"
run phase PRD-002;              expect phase "phase"
run phase-advance PRD-002 --to code; expect_external phase-advance "code|advanced|transition|invalid value"

# ---------------------------------------------------------------------------
# 9. Memory
# ---------------------------------------------------------------------------
# Memory is a text store with substring search, NOT a key-value store — the
# /forge skill claimed otherwise and shipped that claim to every user
# (PROB-094). Exercise the real contract: save a sentence, find it by a word
# inside it, and list everything with a bare `recall`.
run remember "retries are capped at five attempts" --category convention
expect_external remember "Saved|Remembered|remember|stored|mem-"
run recall "capped";            expect "recall finds by substring" "retries are capped"
run recall;                     expect "recall with no query lists all" "retries are capped"

# Whether the DOCS match this is a separate question with its own tool —
# scripts/check-doc-command-drift.sh. Asserting documentation here would put
# two different failures behind one signal.

# ---------------------------------------------------------------------------
# 10. Tags
# ---------------------------------------------------------------------------
run tag PRD-002 "area=payments";   expect_external tag "Tagged|tag|area=payments"
run untag PRD-002 "area=payments"; expect_external untag "Untagged|removed|tag"

# ---------------------------------------------------------------------------
# 11. Index maintenance and data movement
# ---------------------------------------------------------------------------
run export --output "$WORK/backup.json"; expect export "$WORK/backup.json"
run reindex;                    expect_external reindex "indexed|Reindex|artifacts|synced"
run git-sync;                   expect_external git-sync "sync|No |0 |changes"
run scan-import --dry-run;      expect_external scan-import "import|scan|No |0 |dry"
run import "$WORK/backup.json"; expect_external import "import|Imported|skipped|exists"

# ---------------------------------------------------------------------------
# 12. Integration and distribution
# ---------------------------------------------------------------------------
run plugins list;               expect_external "plugins list" "plugin|No |0 |installed"
run mcp install --help;         expect "mcp install --help" "Install"
run setup --skip-model --skip-alias; expect_external setup "Skipping|alias|model|setup"
run setup-skill;                expect_external setup-skill "skill|SKILL.md|installed|forge"
run playbook list;              expect_external "playbook list" "playbook|No |0 |yaml"
run ingest --help;              expect "ingest --help" "mapping"
run discover start harness;     expect_external "discover start" "session|protocol|phase|discover"
run capture "we decided to cap retries at five";  expect_external capture "LLM|API key|provider|not configured"
run promote mem-nonexistent --kind prd;           expect_external promote "not found|no memory|memory"

# ---------------------------------------------------------------------------
# 13. Activity / audit
# ---------------------------------------------------------------------------
run activity;                   expect_external activity "activity|No |0 |tool"
run activity-stats;             expect_external activity-stats "stat|No |0 |tool|count"

# ---------------------------------------------------------------------------
# 14. Release engineering
# ---------------------------------------------------------------------------
run release-notes --help;                expect "release-notes --help" "release notes"
run reconcile-ids;                       expect_external reconcile-ids "drift|No |0 |scanned|clean|reconcile"
run ci-assign-id --help;                 expect "ci-assign-id --help" "assign"

# ---------------------------------------------------------------------------
# 15. Destructive, exercised last
# ---------------------------------------------------------------------------
run delete NOTE-002 --yes;      expect_external delete "Deleted|delete|removed"
run unlink RFC-001 PRD-001 --relation based_on; expect_external unlink "Unlinked|removed|relation"

# ---------------------------------------------------------------------------
# 16. Long-running — structurally cannot be asserted unattended here
# ---------------------------------------------------------------------------
record SKIP serve "stdio MCP server — needs a protocol client, covered separately"
record SKIP watch "filesystem watcher — long-running by design"

# ---------------------------------------------------------------------------
# Report
# ---------------------------------------------------------------------------
echo
echo "──────────────────────────────────────────────────────────────"
printf 'PASS %d   FAIL %d   EXTERNAL %d   SKIP %d\n' "$pass_n" "$fail_n" "$ext_n" "$skip_n"
echo "──────────────────────────────────────────────────────────────"

if [ "$fail_n" -gt 0 ]; then
  echo
  echo "FAILURES:"
  grep '^FAIL' "$RESULTS" | while IFS=$'\t' read -r _ cmd note; do
    printf '  %-22s %s\n' "$cmd" "$note"
  done
fi

if [ "$ext_n" -gt 0 ]; then
  echo
  echo "EXTERNAL (needs a key / model / skill — NOT verified here):"
  grep '^EXTERNAL' "$RESULTS" | while IFS=$'\t' read -r _ cmd note; do
    printf '  %-22s %s\n' "$cmd" "$note"
  done
fi

cp "$RESULTS" "${SURFACE_RESULTS:-$WORK/results.tsv}" 2>/dev/null || true
echo
echo "transcript: $LOG"
echo "results:    $RESULTS"

[ "$fail_n" -eq 0 ]
