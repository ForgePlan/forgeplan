#!/usr/bin/env bash
# .claude/hooks/pre-pr-evidence-check.sh
# Blocks `gh pr create` when artifact lacks linked evidence.
# Exit codes: 0 = proceed, 2 = evidence missing (block)
# Bypass: --no-evidence-check, FORGEPLAN_SKIP_EVIDENCE=1, docs/* branches, chore/sync-* branches

set -euo pipefail

# Color codes for output
RED='\033[0;31m'
YELLOW='\033[1;33m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

# Detect bypass condition: environment variable
if [[ "${FORGEPLAN_SKIP_EVIDENCE:-}" == "1" ]]; then
  exit 0
fi

# Detect bypass condition: branch patterns (mechanical/documentation)
branch=$(git branch --show-current 2>/dev/null || echo "")
case "$branch" in
  docs/*|chore/sync-*|chore/dependabot-*|release/v*|hotfix/*)
    exit 0
    ;;
esac

# Check if current branch is merging to a non-dev/main base
# This would be detected by gh pr create, but we can't know it here
# So we skip if no artifact ref is found at all

# Extract artifact IDs from branch name and recent commits
artifact_ids=""

# From branch name: look for PRD-NNN, RFC-NNN, ADR-NNN, EPIC-NNN, SPEC-NNN, PROB-NNN
branch_artifacts=$(echo "$branch" | grep -oE '(PRD|RFC|ADR|EPIC|SPEC|PROB|EVID|NOTE)-[0-9]+' || true)

# From commit messages (last 20 commits on the current branch)
commit_artifacts=$(git log -20 --pretty='%s%n%b' 2>/dev/null | grep -oE '(PRD|RFC|ADR|EPIC|SPEC|PROB|EVID|NOTE)-[0-9]+' || true)

artifact_ids=$(echo -e "$branch_artifacts\n$commit_artifacts" | sort -u | grep -v '^$')

# If no artifacts mentioned, exit silently (PR may be ad-hoc or non-feature)
if [[ -z "$artifact_ids" ]]; then
  exit 0
fi

# Find forgeplan binary
forgeplan_bin=""
if command -v forgeplan &>/dev/null; then
  forgeplan_bin="forgeplan"
elif command -v fpl &>/dev/null; then
  forgeplan_bin="fpl"
elif [[ -f "./target/debug/forgeplan" ]]; then
  forgeplan_bin="./target/debug/forgeplan"
elif [[ -f "./target/release/forgeplan" ]]; then
  forgeplan_bin="./target/release/forgeplan"
else
  # forgeplan binary not found; soft fail (don't block PR)
  echo -e "${YELLOW}⚠️  forgeplan binary not found; skipping evidence check${NC}" >&2
  exit 0
fi

# Check for evidence links for each artifact
missing_artifacts=()
for artifact_id in $artifact_ids; do
  # Skip checking EVID/NOTE artifacts themselves — they ARE evidence
  case "$artifact_id" in
    EVID-*|NOTE-*)
      continue
      ;;
  esac

  # Query artifact to check for informs or based_on relations
  # Use forgeplan graph or relation query if available
  # Fallback: try to fetch artifact and check body for evidence links

  # Attempt via forgeplan CLI (if available)
  if "$forgeplan_bin" get "$artifact_id" 2>/dev/null | grep -q "informs\|based_on" || \
     "$forgeplan_bin" graph --json 2>/dev/null | grep -q "\"source\":\"$artifact_id\".*\"relation\":\"informs\""; then
    # Evidence found
    :
  else
    # No evidence found — add to missing list
    missing_artifacts+=("$artifact_id")
  fi
done

# If any artifacts lack evidence, block the PR
if (( ${#missing_artifacts[@]} > 0 )); then
  {
    echo ""
    echo -e "${RED}❌ Evidence missing for the following artifacts:${NC}"
    for id in "${missing_artifacts[@]}"; do
      echo "   - $id"
    done
    echo ""
    echo -e "${YELLOW}Each artifact referenced in this PR should have linked evidence (EVID with 'informs' or 'based_on' relation).${NC}"
    echo ""
    echo "Options to fix:"
    echo ""
    echo "  1. Create and link EVID:"
    echo "     forgeplan new evidence \"<title>\" && forgeplan link EVID-XXX <ARTIFACT> --relation informs"
    echo ""
    echo "  2. Use bypass (see docs/methodology/EVIDENCE-PROTOCOL.md for when this is appropriate):"
    echo "     FORGEPLAN_SKIP_EVIDENCE=1 gh pr create ..."
    echo ""
    echo "  Reference: docs/methodology/EVIDENCE-PROTOCOL.md"
    echo ""
  } >&2
  exit 2
fi

exit 0
