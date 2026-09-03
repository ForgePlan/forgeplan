#!/bin/bash
# PreToolUse hook — Forge Mode safety guard
# Blocks destructive commands even in yolo/acceptEdits mode
# FPF B.3: Trust boundary — irreversible actions require human confirmation

# Read tool input from stdin
INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)

# Only check Bash commands
if [ "$TOOL_NAME" != "Bash" ] || [ -z "$COMMAND" ]; then
  exit 0
fi

# BLACKLIST — irreversible/dangerous commands (FPF Red zone)
BLOCKED_PATTERNS=(
  "git push --force"
  "git push -f "
  "git reset --hard"
  "git clean -fd"
  "git checkout -- ."
  "rm -rf /"
  "rm -rf ~"
  "rm -rf \$HOME"
  "drop table"
  "DROP TABLE"
  "cargo publish"
)

for pattern in "${BLOCKED_PATTERNS[@]}"; do
  if echo "$COMMAND" | grep -qF "$pattern"; then
    echo "BLOCKED by forge-safety-hook: '$pattern' detected"
    echo "This is an irreversible action (FPF Red zone). Use manual terminal if intended."
    exit 2
  fi
done

exit 0
