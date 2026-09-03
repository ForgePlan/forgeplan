#!/bin/bash
# PreToolUse hook — Block commit if new public functions lack tests
# FPF Gate: every new pub fn must have a corresponding test

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

# Get staged Rust file changes
DIFF=$(cd "$CLAUDE_PROJECT_DIR" && git diff --cached --unified=0 -- '*.rs' 2>/dev/null)
if [ -z "$DIFF" ]; then
  exit 0
fi

# Find new public functions (added lines with "pub fn" or "pub async fn")
NEW_FNS=$(echo "$DIFF" | grep '^+' | grep -v '^+++' | grep -E 'pub (async )?fn ' | grep -v '#\[test\]' | grep -v '#\[cfg\(test\)\]' | grep -v 'mod tests')

if [ -z "$NEW_FNS" ]; then
  exit 0
fi

# Count new public functions
FN_COUNT=$(echo "$NEW_FNS" | wc -l | tr -d ' ')

# Check if there are also new test functions in the diff
NEW_TESTS=$(echo "$DIFF" | grep '^+' | grep -v '^+++' | grep -E '#\[(tokio::)?test\]')
TEST_COUNT=$(echo "$NEW_TESTS" | grep -c 'test' 2>/dev/null || echo "0")

if [ "$TEST_COUNT" -eq 0 ] && [ "$FN_COUNT" -gt 0 ]; then
  echo "BLOCKED: $FN_COUNT new public function(s) but 0 new tests in this commit."
  echo ""
  echo "New functions without tests:"
  echo "$NEW_FNS" | head -10 | sed 's/^+/  /'
  echo ""
  echo "ACTION: Write tests for each new public function before committing."
  echo "Use /fpf-simple to reason about which tests are needed."
  exit 2
fi

# Warn if ratio is bad (more functions than tests)
if [ "$FN_COUNT" -gt "$TEST_COUNT" ]; then
  echo "WARNING: $FN_COUNT new public functions but only $TEST_COUNT new tests."
  echo "Consider adding more tests. Proceeding anyway."
  # exit 0 = allow with warning
fi

exit 0
