#!/bin/bash
# PreToolUse hook — Block commit if code is not formatted
# Runs cargo fmt --check before any git commit

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)

# Only check git commit commands
if [ "$TOOL_NAME" != "Bash" ] || [ -z "$COMMAND" ]; then
  exit 0
fi
if ! echo "$COMMAND" | grep -qE "git commit"; then
  exit 0
fi

# Check formatting
FMT_OUTPUT=$(cd "$CLAUDE_PROJECT_DIR" && cargo fmt -- --check 2>&1)
if [ $? -ne 0 ]; then
  DIFF_COUNT=$(echo "$FMT_OUTPUT" | grep -c "^Diff in" || echo "0")
  echo "BLOCKED: $DIFF_COUNT file(s) not formatted."
  echo ""
  echo "$FMT_OUTPUT" | head -10
  echo ""
  echo "ACTION: Run 'cargo fmt' before committing."
  exit 2
fi

exit 0
