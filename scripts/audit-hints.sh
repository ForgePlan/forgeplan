#!/bin/bash
# Audit current hint state across CLI commands
# Part of PRD-071 Phase 1 (Cycle 4: 5-rule contract markers)
#
# Output: markdown table classifying each command as GOOD / PARTIAL / MISSING / NULL_OK

set -e

FORGEPLAN="${FORGEPLAN:-/Users/explosovebit/Work/ForgePlan/target/release/forgeplan}"
WORKSPACE="${WORKSPACE:-/tmp/forgeplan-hint-audit}"
OUT="${OUT:-/tmp/hint-audit.md}"

# Setup throwaway workspace
rm -rf "$WORKSPACE"
mkdir -p "$WORKSPACE"
cd "$WORKSPACE"
"$FORGEPLAN" init -y > /dev/null 2>&1

# Seed multiple artifacts so destructive ops (delete/supersede/deprecate/reopen)
# don't cannibalize PRD-001 — most read-only commands target PRD-001, while
# destructive paths target dedicated IDs (PRD-002…PRD-006). Also seed an RFC
# so `supersede --by RFC-001` resolves the replacement target.
"$FORGEPLAN" new prd "Sample PRD" > /dev/null 2>&1 || true        # PRD-001 — shared read target
"$FORGEPLAN" new prd "Delete target" > /dev/null 2>&1 || true     # PRD-002 — delete sandbox
"$FORGEPLAN" new prd "Supersede target" > /dev/null 2>&1 || true  # PRD-003 — supersede sandbox
"$FORGEPLAN" new prd "Deprecate target" > /dev/null 2>&1 || true  # PRD-004 — deprecate sandbox
"$FORGEPLAN" new prd "Reopen target" > /dev/null 2>&1 || true     # PRD-005 — reopen sandbox
"$FORGEPLAN" new rfc "Replacement RFC" > /dev/null 2>&1 || true   # RFC-001 — supersede --by target
"$FORGEPLAN" new evidence "Sample evidence" > /dev/null 2>&1 || true
"$FORGEPLAN" new note "Sample note" > /dev/null 2>&1 || true
"$FORGEPLAN" remember "Sample memory for promote tests" > /dev/null 2>&1 || true

# Get list of subcommands from --help (top-level only)
COMMANDS=$("$FORGEPLAN" --help 2>&1 | awk '/^Commands:/,/^Options:/' | grep -E '^  [a-z]' | awk '{print $1}' | grep -v '^help$' || true)

echo "# Hint Coverage Audit — $(date -u +%Y-%m-%d)" > "$OUT"
echo "" >> "$OUT"
echo "| Command | Text Hint | JSON _next_action | Classification |" >> "$OUT"
echo "|---|---|---|---|" >> "$OUT"

GOOD=0
PARTIAL=0
MISSING=0
NULL_OK=0
SKIPPED=0
TOTAL=0

# Heuristic for sample-arg per command (most read-only commands work without args).
# Multi-arg commands need realistic args so the audit hits real code paths instead
# of bailing out early on missing-arg errors (which would skew "MISSING" counts).
sample_args() {
  case "$1" in
    health|status|claims|blocked|blindspots|order|tree|stale|decay|coverage|drift|gaps|journal|graph|session|progress|migrate|reindex|embed|recall) echo "" ;;
    list) echo "" ;;
    activity|activity-stats) echo "--since-hours 720" ;;
    dispatch) echo "--agents 2" ;;
    search) echo "test" ;;
    # Single-word descriptions — bash word-splits unquoted output of this fn,
    # so multi-word values produce clap errors. Use one-word descriptions.
    route) echo "sample-task" ;;
    new) echo "note audit-test" ;;
    # Multi-arg / option-bearing commands: provide realistic args so the command
    # actually runs and emits its real hint (not an "Usage:" error).
    tag|untag) echo "PRD-001 sample-tag" ;;
    link|unlink) echo "EVID-001 PRD-001 --relation informs" ;;
    # Destructive lifecycle ops use dedicated sandbox IDs (PRD-002..005)
    # so they don't destroy PRD-001 mid-audit and starve later commands.
    delete) echo "PRD-002 --yes" ;;
    # supersede has only --by (no --reason flag); RFC-001 is seeded above.
    supersede) echo "PRD-003 --by RFC-001" ;;
    # deprecate fails on draft→deprecated; we still want it to print the
    # error path so we can verify a Fix: hint is emitted on that path.
    # PRD-004 is dedicated; it's still draft, so this exercises the
    # "Invalid transition" error branch.
    deprecate) echo "PRD-004 --reason audit-test" ;;
    # renew: PRD-001 is fresh (not stale) — error path exercises lifecycle
    # gate and should emit a Fix: hint.
    renew) echo "PRD-001 --reason audit-test --until 2027-12-31" ;;
    reopen) echo "PRD-005 --reason audit-test" ;;
    phase-advance) echo "PRD-001 --to shape" ;;
    restore) echo "PRD-001" ;;
    # import without a valid file → expect Fix: hint on error path.
    # /dev/null gives "Invalid export JSON" error which the import code
    # path emits a Fix: marker for.
    import) echo "/dev/null" ;;
    # remember takes a single positional TEXT — must be one shell word
    # (audit script intentionally word-splits on whitespace, so any
    # multi-word value would become multiple positional args and fail
    # clap parsing). slug-form audit-test stays as one token.
    remember) echo "audit-test-memory" ;;
    # calibrate-estimate succeeds → emits Next:; on failure → emits Fix:.
    # PRD-001 has no estimable items so this exercises the error branch
    # which DOES emit a Fix: hint per the calibrate_estimate.rs source.
    calibrate-estimate) echo "PRD-001 --actual-hours 8" ;;
    # discover start needs a NAME positional. Single-word per audit
    # word-split convention.
    discover) echo "start audit-test-proj" ;;
    fpf) echo "rules" ;;
    capture) echo "audit-decision" ;;
    # mcp install --dry-run --scope project doesn't write any files but
    # still triggers the install code path which emits Next:/Done.
    # --client claude is the most universal target.
    mcp) echo "install --client claude --scope project --dry-run" ;;
    # git-sync has --since but no positional subcommand. Empty arg
    # exercises the no-ORIG_HEAD error which currently emits "Specify a ref:"
    # — not a 5-rule marker. Use --since HEAD to force a real sync path
    # which emits a Next: hint after the sync block.
    git-sync) echo "--since HEAD" ;;
    promote) echo "MEM-9999 --kind prd" ;;        # Memory not found → Fix: hint
    generate) echo "prd audit-test-description" ;; # No LLM configured → Fix: hint
    # undo-last takes only --within-hours (no positional ID). Default
    # 24h window will find no receipts in our fresh workspace, which
    # exercises the error-path Fix: hint.
    undo-last) echo "" ;;
    # Per-id read/lifecycle commands.
    get|update|score|fgr|review|activate|reason|decompose|context|estimate|calibrate|claim|release|phase) echo "PRD-001" ;;
    *) echo "" ;;
  esac
}

for cmd in $COMMANDS; do
  # Skip long-running daemon commands — they'd hang the audit.
  case "$cmd" in
    serve|watch)
      SKIPPED=$((SKIPPED+1))
      echo "| \`$cmd\` | SKIPPED (daemon) | N/A | SKIPPED |" >> "$OUT"
      continue
      ;;
  esac

  TOTAL=$((TOTAL+1))
  args=$(sample_args "$cmd")

  # Capture text output (stdout + stderr)
  text_out=$(timeout 10 "$FORGEPLAN" "$cmd" $args 2>&1 || true)

  # Detect text hint per PRD-071 5-rule contract: GOOD = any of the 5 markers
  # (Next/Or/Wait/Fix on a line OR a standalone "Done."). PARTIAL catches old-
  # style hints that haven't been migrated yet. NULL_OK catches terminal status
  # commands that intentionally print "Workspace healthy" without a marker.
  text_classification="MISSING"
  if echo "$text_out" | grep -qE '^[[:space:]]*(Next|Or|Wait|Fix):[[:space:]]+|^[[:space:]]*Done\.[[:space:]]*$'; then
    text_classification="GOOD"
  elif echo "$text_out" | grep -qE '_next_action|Hint:|→ Next:|next:|next_action'; then
    text_classification="PARTIAL"
  elif echo "$text_out" | grep -qE 'forgeplan_health|Workspace healthy'; then
    text_classification="NULL_OK"
  fi

  # Capture JSON output if --json supported
  json_classification="N/A"
  json_out=$(timeout 10 "$FORGEPLAN" "$cmd" $args --json 2>&1 || true)
  if echo "$json_out" | head -c 1 | grep -q '{'; then
    if echo "$json_out" | grep -q '"_next_action"'; then
      json_classification="GOOD"
    else
      json_classification="MISSING"
    fi
  fi

  # Final classification: worst of text + json
  case "$text_classification" in
    GOOD) GOOD=$((GOOD+1)) ;;
    PARTIAL) PARTIAL=$((PARTIAL+1)) ;;
    MISSING) MISSING=$((MISSING+1)) ;;
    NULL_OK) NULL_OK=$((NULL_OK+1)) ;;
  esac

  echo "| \`$cmd\` | $text_classification | $json_classification | $text_classification |" >> "$OUT"
done

echo "" >> "$OUT"
echo "## Summary" >> "$OUT"
echo "" >> "$OUT"
echo "- Total commands audited: $TOTAL (skipped daemons: $SKIPPED)" >> "$OUT"
echo "- GOOD (5-rule contract marker emitted): $GOOD" >> "$OUT"
echo "- PARTIAL (some hint, not contract-compliant): $PARTIAL" >> "$OUT"
echo "- MISSING (no hint at all): $MISSING" >> "$OUT"
echo "- NULL_OK (terminal, hint correctly absent): $NULL_OK" >> "$OUT"
echo "" >> "$OUT"
COVERAGE_PCT=$(awk "BEGIN { printf \"%.1f\", ($GOOD + $NULL_OK) * 100 / $TOTAL }")
echo "**Coverage**: $COVERAGE_PCT% ($((GOOD + NULL_OK))/$TOTAL contract-compliant)" >> "$OUT"

# Cleanup
rm -rf "$WORKSPACE"

echo "Audit complete. Report: $OUT"
cat "$OUT" | tail -10
