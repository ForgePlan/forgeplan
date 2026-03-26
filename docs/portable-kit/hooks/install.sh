#!/bin/bash
# Установка enforcement hooks для Claude Code
# Запускать из корня проекта: bash .project-kit/hooks/install.sh

set -e

HOOKS_DIR=".claude/hooks"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Installing enforcement hooks..."

mkdir -p "$HOOKS_DIR"

for hook in forge-safety.sh pr-todo-check.sh commit-test.sh pre-code-check.sh pre-commit-health.sh; do
  cp "$SCRIPT_DIR/$hook" "$HOOKS_DIR/$hook"
  chmod +x "$HOOKS_DIR/$hook"
  echo "  + $hook"
done

# Создать settings.json если нет
SETTINGS=".claude/settings.json"
if [ ! -f "$SETTINGS" ]; then
  cp "$SCRIPT_DIR/../settings-template.json" "$SETTINGS"
  echo "  + settings.json (from template)"
else
  echo "  ~ settings.json already exists — add hooks manually"
  echo "    See settings-template.json for reference"
fi

echo ""
echo "Done! 5 hooks installed in $HOOKS_DIR"
echo ""
echo "Hooks active:"
echo "  forge-safety     — blocks dangerous commands"
echo "  pr-todo-check    — requires P0 checkboxes before PR"
echo "  commit-test      — requires tests for new functions"
echo "  pre-code-check   — requires active PRD before code edits"
echo "  pre-commit-health — warns about blind spots"
