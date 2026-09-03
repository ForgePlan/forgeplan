#!/bin/bash
# PreToolUse hook — FR-005: Pre-commit health check
# Warns (does not block) if forgeplan health reports blind spots
# Blind spots = active artifacts without evidence (R_eff = 0)

# Read tool input from stdin
INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)

# Only check Bash commands containing "git commit"
if [ "$TOOL_NAME" != "Bash" ] || [ -z "$COMMAND" ]; then
  exit 0
fi
if ! echo "$COMMAND" | grep -qE "git commit"; then
  exit 0
fi

# Run forgeplan health --compact
HEALTH_OUTPUT=$(cd "$CLAUDE_PROJECT_DIR" 2>/dev/null && forgeplan health --compact 2>/dev/null)
if [ $? -ne 0 ] || [ -z "$HEALTH_OUTPUT" ]; then
  # No workspace or forgeplan not available — skip check
  exit 0
fi

# Parse "Blind spots: N" from output
BLIND_SPOTS=$(echo "$HEALTH_OUTPUT" | grep -oE 'Blind spots: [0-9]+' | grep -oE '[0-9]+')

if [ -z "$BLIND_SPOTS" ] || [ "$BLIND_SPOTS" = "0" ]; then
  exit 0
fi

echo "WARNING: forgeplan health reports $BLIND_SPOTS blind spot(s)."
echo ""
echo "Blind spots = active artifacts without evidence (R_eff = 0)."
echo "Consider adding evidence before committing:"
echo ""
echo "  forgeplan health                    # see details"
echo "  forgeplan new evidence 'Description'"
echo "  forgeplan link EVID-XXX PRD-XXX --relation informs"
echo ""
echo "You can proceed with this commit, but blind spots should be addressed."
# exit 1 = WARNING (user can override)
exit 1
