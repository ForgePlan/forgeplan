# PRD-001: CLI UX Redesign — cliclack interactive UI

## Problem

Текущий CLI выводит plain text без стилизации. `forgeplan init` создаёт пустые папки молча. `forgeplan health` выглядит как debug output. Нет интерактивности — пользователь не выбирает агентов, не подтверждает действия. Первое впечатление = "сырой CLI", а не "продуманный developer tool".

## Goals

- Интерактивный `forgeplan init` с выбором агентов, генерацией .mcp.json, ASCII banner
- Стилизованный output для health, validate, review, route, fgr, list
- Единый визуальный стиль через cliclack
- Сохранить --json flag для machine-readable output

## Non-Goals

- Не менять MCP server output (остаётся JSON)
- Не менять логику команд (только presentation layer)
- Не делать TUI/dashboard (Phase 5)

## Target Users

Разработчики настраивающие Forgeplan и проверяющие состояние проекта через CLI.

## Functional Requirements

- [x] FR-001: ASCII banner FPL при init
- [x] FR-002: Interactive wizard: name → agents → .mcp.json → spinner → summary
- [x] FR-003: .mcp.json auto-generation для Claude Code/Cursor
- [ ] FR-004: CLAUDE.md section auto-generation
- [x] FR-005: .cursorrules auto-generation
- [ ] FR-006: health — styled output с note boxes, цветными статусами
- [ ] FR-007: validate — цветные MUST/SHOULD/COULD
- [ ] FR-008: review — styled checklist
- [ ] FR-009: route — styled depth colors
- [ ] FR-010: list — colored table by status
- [ ] FR-011: --json flag на всех командах
- [ ] FR-012: setup-skill command

## Related Artifacts

Продолжение работы после EPIC-001 (v0.7.0).

## Implementation Notes

cliclack = "0.5", console = "0.15". Audit passed (6.8/10 → fixes applied).
Branch: feat/prd-008-cli-ux. 4/12 FR done + audit fixes.
