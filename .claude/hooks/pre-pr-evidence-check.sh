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

# Narrow to `gh pr create` ONLY. Anything else passes silently.
if [[ -z "$command_str" ]] || ! [[ "$command_str" =~ (^|[[:space:]\;\&\|])gh[[:space:]]+pr[[:space:]]+create([[:space:]]|$) ]]; then
  exit 0
fi

# --no-evidence-check flag on the gh pr create command line → bypass
if [[ "$command_str" =~ --no-evidence-check ]]; then
  exit 0
fi

# Env-var bypass (must be exported before the gh call, e.g.
# `FORGEPLAN_SKIP_EVIDENCE=1 gh pr create ...` — Claude Code propagates env)
if [[ "${FORGEPLAN_SKIP_EVIDENCE:-}" == "1" ]]; then
  exit 0
fi
# Also detect inline env-var assignment in the command itself.
if [[ "$command_str" =~ FORGEPLAN_SKIP_EVIDENCE=1 ]]; then
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
  if command -v jq &>/dev/null; then
    graph_json=$("$forgeplan_bin" graph --json 2>/dev/null || echo "")
    if [[ -n "$graph_json" ]]; then
      # The graph schema may use `from`/`to` or `source`/`target`; try both.
      # Match: any edge whose source == artifact_id, kind == evidence, relation in (informs, based_on).
      match=$(echo "$graph_json" | jq -r --arg id "$artifact_id" '
        ([.edges[]?, .relations[]?] | map(select(. != null))) as $edges
        | $edges
        | map(select(
            ((.source // .from) == $id) and
            ((.relation // .kind) == "informs" or (.relation // .kind) == "based_on") and
            ((.target_kind // .kind // "") | tostring | test("evidence"; "i"))
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
      relations_section=$(echo "$body" | awk '/^## (Related|Links|Relations)/,/^## /' || true)
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
