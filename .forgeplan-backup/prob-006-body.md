# PROB-006: Routing misses UX/redesign scope

## Signal

`forgeplan route "Redesign CLI with cliclack UI"` → Tactical (confidence 80%).
Реально это Standard+ задача: новая зависимость, 33 команды затронуты, 1-3 дня работы.

## Root Cause

Keyword triggers в routing/signals.rs не содержат "redesign", "overhaul", "refactor all", "new dependency", "UX". Эти слова не в списке escalation triggers.

## Fix

Добавить keyword triggers:
- `redesign`, `overhaul`, `rewrite` → Standard+
- `new dependency`, `new crate`, `new library` → Standard+
- `all commands`, `entire CLI` → Standard+ (scope signal)
