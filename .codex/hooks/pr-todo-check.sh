#!/bin/bash
# PreToolUse hook — Block PR if P0 TODO items are unchecked
# FPF Gate → Reasoning Loop: block triggers /fpf-simple for decision
#
# Exit codes:
#   0 = pass (all P0 checked)
#   2 = BLOCK with /fpf-simple instruction

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)

# Only check Bash commands that create PRs
if [ "$TOOL_NAME" != "Bash" ] || [ -z "$COMMAND" ]; then
  exit 0
fi

# Match: gh pr create
if ! echo "$COMMAND" | grep -qE "gh pr (create|submit)"; then
  exit 0
fi

# Check TODO.md for unchecked P0 items
TODO_FILE="${CLAUDE_PROJECT_DIR}/TODO.md"
if [ ! -f "$TODO_FILE" ]; then
  exit 0
fi

# Find ALL unchecked P0 items (between "### P0" and next "### P" or "---")
P0_ITEMS=$(awk '/^### P0/,/^### P[1-9]|^---/{print}' "$TODO_FILE" | grep '\- \[ \]' 2>/dev/null || true)
if [ -z "$P0_ITEMS" ]; then
  P0_COUNT=0
else
  P0_COUNT=$(echo "$P0_ITEMS" | wc -l | tr -d ' ')
fi

if [ "$P0_COUNT" -gt 0 ]; then
  echo "BLOCKED: $P0_COUNT unchecked P0 item(s) in TODO.md."
  echo ""
  echo "Unchecked P0 items:"
  echo "$P0_ITEMS"
  echo ""
  echo "ACTION REQUIRED: Use /fpf-simple to reason about each unchecked item:"
  echo "  For each item, evaluate 3 alternatives:"
  echo "    A) Complete it now (implement + test + mark [x])"
  echo "    B) Mark done [x] if already implemented but checkbox not updated"
  echo "    C) Downgrade to P1 if out of scope for this PR (with justification)"
  echo ""
  echo "  Then follow methodology: Evidence → TODO update → retry PR"
  echo ""
  echo "  Invoke: /fpf-simple \"Evaluate P0 items blocking PR: <list items>\""
  exit 2
fi

exit 0
