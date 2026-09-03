#!/bin/bash
# PreToolUse hook — FR-004: Pre-code check
# Blocks Edit/Write on crates/ if no active PRD exists (Standard+ depth)
# Shape → Validate → Code: ensure PRD is active before coding

# Read tool input from stdin
INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)

# Only check Edit and Write tool calls
if [ "$TOOL_NAME" != "Edit" ] && [ "$TOOL_NAME" != "Write" ]; then
  exit 0
fi

# Get the file path from tool input
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null)

# Only check files in crates/ directory
if ! echo "$FILE_PATH" | grep -q "crates/"; then
  exit 0
fi

# Check if forgeplan workspace exists (health --compact --json)
HEALTH=$(cd "$CLAUDE_PROJECT_DIR" 2>/dev/null && forgeplan health --compact --json 2>/dev/null)
if [ $? -ne 0 ] || [ -z "$HEALTH" ]; then
  # No workspace or forgeplan not available — skip check
  exit 0
fi

# Check artifact count — if 0, no workspace data, skip
ARTIFACT_COUNT=$(echo "$HEALTH" | jq -r '.artifact_count // 0' 2>/dev/null)
if [ "$ARTIFACT_COUNT" = "0" ] || [ -z "$ARTIFACT_COUNT" ]; then
  exit 0
fi

# Check for active PRDs
ACTIVE_PRDS=$(cd "$CLAUDE_PROJECT_DIR" 2>/dev/null && forgeplan list --kind prd --json 2>/dev/null | jq '[.[] | select(.status=="active")] | length' 2>/dev/null)

if [ -z "$ACTIVE_PRDS" ] || [ "$ACTIVE_PRDS" = "0" ]; then
  echo "BLOCKED: No active PRD found."
  echo ""
  echo "Shape → Validate → Code: create and activate a PRD before editing code in crates/."
  echo ""
  echo "  forgeplan new prd 'Title'"
  echo "  # Fill MUST sections: Problem, Goals, Non-Goals, Target Users, Related, FR"
  echo "  forgeplan validate PRD-XXX"
  echo "  forgeplan activate PRD-XXX"
  exit 2
fi

exit 0
