#!/usr/bin/env bash
# .claude/hooks/pre-pr-evidence-check.sh
#
# Triggered by Claude Code PreToolUse hook on Bash invocations.
# Inspects the proposed command (passed as JSON on stdin per Claude Code hook
# protocol). Blocks only when the command is `gh pr create ...` AND the branch
# carries artifact refs without linked evidence.
#
# Exit codes:
#   0 — proceed (not a pr-create, bypass active, evidence present, or no artifact ref)
#   2 — block (artifact refs present but no linked evidence)
#
# Bypass paths (all give exit 0):
#   - FORGEPLAN_SKIP_EVIDENCE=1 in env passed to gh
#   - Branch matches: docs/*, chore/sync-*, chore/dependabot-*, release/v*, hotfix/*
#   - `gh pr create` invocation includes `--no-evidence-check` flag in argv
#   - The bash command is not `gh pr create` at all
#
# Round-2 audit fixes:
#   - SEC-M-R2-2: only fires on actual `gh pr create`; greedy grep replaced
#     with jq-based exact match on canonical graph JSON shape
#   - CRIT-2 (code review): wired to PreToolUse via .claude/settings.json

set -euo pipefail

RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Read the Claude Code hook payload from stdin (JSON with `tool_input.command`)
# If stdin is empty or non-JSON we fall back to "not a gh pr create" → exit 0.
payload=$(cat 2>/dev/null || true)
if [[ -z "$payload" ]]; then
  exit 0
fi

# Extract the proposed Bash command. Without jq we get false negatives, so try jq
# first, then fall back to a tolerant grep that still narrows to gh-pr-create.
if command -v jq &>/dev/null; then
  command_str=$(echo "$payload" | jq -r '.tool_input.command // empty' 2>/dev/null || true)
else
  command_str=$(echo "$payload" | grep -oE '"command"[[:space:]]*:[[:space:]]*"[^"]+"' | head -1 | sed 's/.*"command"[[:space:]]*:[[:space:]]*"//;s/"$//' || true)
fi

# Narrow to `gh pr create` ONLY — and only when it is the actual command
# being executed, NOT when the literal string appears inside an argument
# of another command (e.g. a `git commit -m '... gh pr create ...'`
# whose commit message documents the command in prose).
#
# S3 audit closure follow-on: the previous regex accepted `gh pr create`
# anywhere it was preceded by `(^|space|;|&|\|)`, which fires false-
# positively whenever the literal text appears inside a quoted heredoc
# or string argument. To narrow correctly we must look at the FIRST
# real command in the line, after stripping leading env-var assignments
# (FOO=bar BAZ=1) and whitespace. That command must literally be
# `gh pr create`.
if [[ -z "$command_str" ]]; then
  exit 0
fi

stripped="$command_str"
# Drop leading whitespace.
stripped="${stripped#"${stripped%%[![:space:]]*}"}"
# Strip any number of leading SHELLVAR=value assignments (with their
# trailing whitespace). The regex matches one assignment + its trailing
# whitespace; loop until none remain.
while [[ "$stripped" =~ ^[A-Za-z_][A-Za-z_0-9]*=[^[:space:]]*[[:space:]]+ ]]; do
  stripped="${stripped#"${BASH_REMATCH[0]}"}"
done

# The first command tokens must be `gh pr create` (with any whitespace).
if ! [[ "$stripped" =~ ^gh[[:space:]]+pr[[:space:]]+create([[:space:]]|$) ]]; then
  exit 0
fi

# S3 audit closure: split the command around the `gh pr create` token so we
# can apply bypass detection token-aware-ly instead of substring-naive-ly.
#
# Previously both bypass tokens were matched against the entire reconstructed
# command string. Anything containing the literal substring — including the
# value of `--body "..."`, `--title "..."`, or label content — triggered a
# silent bypass. Concrete repros from the security audit:
#   gh pr create --body "see --no-evidence-check ADR"     → silent bypass
#   gh pr create --body "FORGEPLAN_SKIP_EVIDENCE=1 hack"  → silent bypass
#
# The two regions we care about:
#   ENV_PREFIX — everything BEFORE the first `gh ` token. This is where shell
#                env-var assignments live syntactically (`FOO=1 gh ...`).
#   ARGV_HEAD  — everything AFTER `gh pr create` up to the FIRST quote
#                character. This is the unquoted flag region; quoted argument
#                values (--body, --title) are excluded by construction.
env_prefix="${command_str%%gh pr create*}"
post_gh="${command_str#*gh pr create}"
argv_head="${post_gh%%[\"\']*}"

# --no-evidence-check flag → bypass. Must be a standalone token in argv_head
# (the unquoted flag region), not buried in a quoted body or title.
if [[ "$argv_head" =~ (^|[[:space:]])--no-evidence-check([[:space:]]|$) ]]; then
  exit 0
fi

# Env-var bypass (must be exported before the gh call, e.g.
# `FORGEPLAN_SKIP_EVIDENCE=1 gh pr create ...` — Claude Code propagates env)
if [[ "${FORGEPLAN_SKIP_EVIDENCE:-}" == "1" ]]; then
  exit 0
fi
# Also detect inline env-var assignment, but ONLY in the env-prefix region
# (before `gh pr create`). A mention of the var inside --body is prose,
# not a bypass.
if [[ "$env_prefix" =~ (^|[[:space:]\;\&\|])FORGEPLAN_SKIP_EVIDENCE=1([[:space:]]|$) ]]; then
  exit 0
fi

# Branch-pattern bypass.
branch=$(git branch --show-current 2>/dev/null || echo "")
case "$branch" in
  docs/*|chore/sync-*|chore/dependabot-*|release/v*|hotfix/*)
    exit 0
    ;;
esac

# Collect artifact IDs referenced in branch + last 20 commits.
branch_ids=$(echo "$branch" | grep -oE '(PRD|RFC|ADR|EPIC|SPEC|PROB|EVID|NOTE)-[0-9]+' || true)
commit_ids=$(git log -20 --pretty='%s%n%b' 2>/dev/null | grep -oE '(PRD|RFC|ADR|EPIC|SPEC|PROB|EVID|NOTE)-[0-9]+' || true)
artifact_ids=$(printf '%s\n%s\n' "$branch_ids" "$commit_ids" | sort -u | grep -v '^$' || true)

# No artifacts referenced → can't enforce evidence; pass.
if [[ -z "$artifact_ids" ]]; then
  exit 0
fi

# Find forgeplan binary.
forgeplan_bin=""
if command -v forgeplan &>/dev/null; then
  forgeplan_bin="forgeplan"
elif command -v fpl &>/dev/null; then
  forgeplan_bin="fpl"
elif [[ -x "./target/debug/forgeplan" ]]; then
  forgeplan_bin="./target/debug/forgeplan"
elif [[ -x "./target/release/forgeplan" ]]; then
  forgeplan_bin="./target/release/forgeplan"
else
  echo -e "${YELLOW}⚠️  pre-pr-evidence-check: forgeplan binary not found; skipping check${NC}" >&2
  exit 0
fi

# Check evidence links for each non-evidence artifact.
missing_artifacts=()
for artifact_id in $artifact_ids; do
  case "$artifact_id" in
    EVID-*|NOTE-*)
      continue
      ;;
  esac

  has_evidence=0

  # Primary path: structured query on graph JSON via jq (exact JSON-shape match,
  # avoids the greedy-grep bug Round-2 audit caught — `.*` could cross record
  # boundaries when relations are checked positionally).
  #
  # FIX (dogfood discovery 2026-05-21): the previous query looked for edges
  # where `source == artifact_id` with `target_kind == evidence`. That's the
  # WRONG direction — evidence pattern is `EVID-NNN --informs--> ARTIFACT`,
  # so the EVID is the SOURCE, the artifact is the TARGET. The old query
  # matched zero edges and blocked every PR. Corrected logic: find any edge
  # where `target == artifact_id`, `source` starts with `EVID-`, relation
  # in (informs, based_on).
  if command -v jq &>/dev/null; then
    graph_json=$("$forgeplan_bin" graph --json 2>/dev/null || echo "")
    if [[ -n "$graph_json" ]]; then
      # The graph schema may use `from`/`to` or `source`/`target`; try both.
      # Match: any edge where TARGET == artifact_id, SOURCE is an EVID-*,
      # relation in (informs, based_on). This pins the correct direction.
      match=$(echo "$graph_json" | jq -r --arg id "$artifact_id" '
        ([.edges[]?, .relations[]?] | map(select(. != null))) as $edges
        | $edges
        | map(select(
            ((.target // .to) == $id) and
            ((.source // .from) | tostring | startswith("EVID-")) and
            ((.relation // .kind) == "informs" or (.relation // .kind) == "based_on")
          ))
        | length
      ' 2>/dev/null || echo 0)
      if [[ "$match" -gt 0 ]]; then
        has_evidence=1
      fi
    fi
  fi

  # Fallback path: heuristic body-text scan, but scoped to the relations
  # section so prose mentions of "informs" don't cause false positives.
  if [[ "$has_evidence" -eq 0 ]]; then
    body=$("$forgeplan_bin" get "$artifact_id" 2>/dev/null || echo "")
    if [[ -n "$body" ]]; then
      # Audit-r-release F4 (code-reviewer): the range syntax
      # `/start/,/end/` matches inclusive on both endpoints — when the
      # start pattern also matches the end pattern (both begin with
      # `^## `), the range collapses to a single line. Use a flag-based
      # awk script so we skip the start marker and terminate cleanly on
      # the next heading.
      relations_section=$(echo "$body" | awk '/^## (Related|Links|Relations)/{flag=1; next} /^## /{flag=0} flag' || true)
      if echo "$relations_section" | grep -qE '(EVID-[0-9]+|evidence|informs|based_on)'; then
        has_evidence=1
      fi
    fi
  fi

  if [[ "$has_evidence" -eq 0 ]]; then
    missing_artifacts+=("$artifact_id")
  fi
done

# Decision.
if (( ${#missing_artifacts[@]} > 0 )); then
  {
    echo ""
    echo -e "${RED}❌ Cannot create PR — evidence missing for:${NC}"
    for id in "${missing_artifacts[@]}"; do
      echo "   - $id"
    done
    echo ""
    echo -e "${YELLOW}Each artifact referenced in this PR's branch or commits${NC}"
    echo -e "${YELLOW}must have linked evidence (EVID with 'informs' or 'based_on' relation).${NC}"
    echo ""
    echo "Options:"
    echo "  1. Create + link evidence:"
    echo "     forgeplan new evidence \"<title>\""
    echo "     forgeplan link EVID-XXX <ARTIFACT-ID> --relation informs"
    echo "     forgeplan activate EVID-XXX"
    echo ""
    echo "  2. Bypass (only for docs-only PRs, hotfixes, dependabot syncs):"
    echo "     FORGEPLAN_SKIP_EVIDENCE=1 gh pr create ..."
    echo "     OR add --no-evidence-check to the gh pr create command line"
    echo ""
    echo "  Document bypass justification in PR body — see docs/methodology/EVIDENCE-PROTOCOL.md"
    echo ""
  } >&2
  exit 2
fi

exit 0
