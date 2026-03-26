#!/bin/bash
# PreToolUse hook — предупреждает о blind spots перед коммитом
# Matcher: Bash (git commit)
# Exit 1 = warning (можно продолжить), exit 2 = block

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)

if [ "$TOOL_NAME" != "Bash" ] || [ -z "$COMMAND" ]; then
  exit 0
fi
if ! echo "$COMMAND" | grep -qE "git commit"; then
  exit 0
fi

if ! command -v forgeplan &> /dev/null; then
  exit 0
fi

HEALTH=$(forgeplan health --compact 2>/dev/null || echo "")
if [ -z "$HEALTH" ]; then
  exit 0
fi

BLIND=$(echo "$HEALTH" | grep -oP 'Blind spots: \K[0-9]+' 2>/dev/null || echo "0")

if [ "$BLIND" -gt 0 ]; then
  echo "WARNING: $BLIND blind spot(s) — artifacts without evidence."
  echo "$HEALTH"
  echo ""
  echo "Consider creating evidence before committing."
  exit 1  # warning, not block
fi

exit 0
