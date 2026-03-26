#!/bin/bash
# PreToolUse hook — блокирует опасные команды
# Matcher: Bash

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)

if [ "$TOOL_NAME" != "Bash" ] || [ -z "$COMMAND" ]; then
  exit 0
fi

BLOCKED_PATTERNS=(
  "git push --force"
  "git push -f "
  "git reset --hard"
  "git clean -fd"
  "rm -rf /"
  "rm -rf ~"
  "rm -rf \$HOME"
  "DROP TABLE"
  "drop table"
)

for pattern in "${BLOCKED_PATTERNS[@]}"; do
  if echo "$COMMAND" | grep -qF "$pattern"; then
    echo "BLOCKED: '$pattern' detected. This is irreversible."
    exit 2
  fi
done

exit 0
