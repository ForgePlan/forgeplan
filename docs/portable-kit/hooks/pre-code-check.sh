#!/bin/bash
# PreToolUse hook — требует active PRD перед редактированием кода
# Matcher: Bash (Edit/Write на src/)
# Адаптируй SRC_PATTERN под свой проект

INPUT=$(cat)
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)

if [ "$TOOL_NAME" != "Edit" ] && [ "$TOOL_NAME" != "Write" ]; then
  exit 0
fi

FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null)

# Адаптируй: какие папки считаются "кодом"
# Rust: crates/, src/
# TypeScript: src/, packages/
# Python: src/, app/
SRC_PATTERN="crates/\|src/"

if ! echo "$FILE_PATH" | grep -q "$SRC_PATTERN"; then
  exit 0
fi

# Проверяем есть ли forgeplan workspace
if ! command -v forgeplan &> /dev/null; then
  exit 0
fi

HEALTH=$(forgeplan health --compact --json 2>/dev/null || echo "")
if [ -z "$HEALTH" ]; then
  exit 0
fi

# Проверяем есть ли active PRD
ACTIVE_PRDS=$(forgeplan list --kind prd --json 2>/dev/null | jq '[.[] | select(.status=="active")] | length' 2>/dev/null || echo "0")

if [ "$ACTIVE_PRDS" -eq 0 ]; then
  echo "BLOCKED: No active PRD. Create one before coding:"
  echo "  forgeplan new prd 'What you are building'"
  echo "  forgeplan activate PRD-XXX"
  exit 2
fi

exit 0
