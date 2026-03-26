#!/bin/bash
# PreToolUse hook — блокирует коммит если новые pub fn без тестов
# Matcher: Bash (git commit)

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)
COMMAND=$(echo "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)

if [ "$TOOL_NAME" != "Bash" ] || [ -z "$COMMAND" ]; then
  exit 0
fi
if ! echo "$COMMAND" | grep -qE "git commit"; then
  exit 0
fi

# Ищем новые публичные функции в staged .rs файлах
DIFF=$(cd "$CLAUDE_PROJECT_DIR" && git diff --cached --unified=0 -- '*.rs' 2>/dev/null)
if [ -z "$DIFF" ]; then
  exit 0
fi

NEW_FNS=$(echo "$DIFF" | grep '^+' | grep -v '^+++' | grep -E 'pub (async )?fn ' | grep -v '#\[test\]' | grep -v 'mod tests')
if [ -z "$NEW_FNS" ]; then
  exit 0
fi

FN_COUNT=$(echo "$NEW_FNS" | wc -l | tr -d ' ')
NEW_TESTS=$(echo "$DIFF" | grep '^+' | grep -v '^+++' | grep -E '#\[(tokio::)?test\]')
TEST_COUNT=0
if [ -n "$NEW_TESTS" ]; then
  TEST_COUNT=$(echo "$NEW_TESTS" | grep -c 'test' 2>/dev/null || echo "0")
fi

if [ "$TEST_COUNT" -eq 0 ] && [ "$FN_COUNT" -gt 0 ]; then
  echo "BLOCKED: $FN_COUNT new public function(s) but 0 new tests."
  echo ""
  echo "$NEW_FNS" | head -10 | sed 's/^+/  /'
  echo ""
  echo "Write tests for each new function before committing."
  exit 2
fi

exit 0
