#!/bin/bash
# PreToolUse hook — блокирует PR если есть незакрытые P0 в TODO.md
# Matcher: Bash (gh pr create)

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)

if [ "$TOOL_NAME" != "Bash" ] || [ -z "$COMMAND" ]; then
  exit 0
fi

if ! echo "$COMMAND" | grep -qE "gh pr (create|submit)"; then
  exit 0
fi

TODO_FILE="${CLAUDE_PROJECT_DIR}/TODO.md"
if [ ! -f "$TODO_FILE" ]; then
  exit 0
fi

P0_ITEMS=$(awk '/^### P0/,/^### P[1-9]|^---/{print}' "$TODO_FILE" | grep '\- \[ \]' 2>/dev/null || true)
if [ -z "$P0_ITEMS" ]; then
  exit 0
fi

P0_COUNT=$(echo "$P0_ITEMS" | wc -l | tr -d ' ')

echo "BLOCKED: $P0_COUNT unchecked P0 item(s) in TODO.md."
echo ""
echo "$P0_ITEMS"
echo ""
echo "Complete P0 items or mark [x] before creating PR."
exit 2
