# PRD-008: CLI UX Redesign — cliclack interactive UI

## Problem

Текущий CLI выводит plain text без стилизации. `forgeplan init` создаёт пустые папки молча. `forgeplan health` выглядит как debug output. Нет интерактивности — пользователь не выбирает агентов, не подтверждает действия. Для MCP-first инструмента это не критично (агент читает JSON), но для human onboarding и inspection — плохо. Первое впечатление = "сырой CLI", а не "продуманный developer tool". Сравнение с npx skills (cliclack UI) делает разницу очевидной.

## Goals

- Интерактивный `forgeplan init` с выбором агентов, генерацией .mcp.json, ASCII banner
- Стилизованный output для health, validate, review, route, fgr, list
- Единый визуальный стиль через cliclack (vertical timeline для interactive, note boxes для output)
- Сохранить --json flag для machine-readable output (MCP и scripting)

## Non-Goals

- Не менять MCP server output (остаётся JSON)
- Не менять логику команд (только presentation layer)
- Не делать TUI/dashboard (это для Desktop App, Phase 5)

## Target Users

- Разработчики впервые настраивающие Forgeplan (onboarding)
- Люди проверяющие состояние проекта через CLI (inspection)
- AI агенты НЕ затронуты (MCP output = JSON, без изменений)

## Functional Requirements

- [ ] FR-001: ASCII banner FPL при `init` и `--version`
- [ ] FR-002: `forgeplan init` — интерактивный wizard: project name → agent multiselect → .mcp.json confirm → spinner → summary note → outro
- [ ] FR-003: `forgeplan init` генерирует .mcp.json для выбранных агентов
- [ ] FR-004: `forgeplan init` добавляет Forgeplan секцию в CLAUDE.md (если выбран Claude Code)
- [ ] FR-005: `forgeplan init` генерирует .cursorrules (если выбран Cursor)
- [ ] FR-006: `forgeplan health` — styled output с note boxes, цветными статусами, иконками
- [ ] FR-007: `forgeplan validate` — цветные MUST (красный) / SHOULD (жёлтый) / COULD (серый)
- [ ] FR-008: `forgeplan review` — styled checklist с цветами
- [ ] FR-009: `forgeplan route` — styled result с depth цветом (tactical=зелёный, deep=красный)
- [ ] FR-010: `forgeplan list` — styled table с цветами по status (active=зелёный, draft=серый)
- [ ] FR-011: Все команды поддерживают `--json` для machine-readable output
- [ ] FR-012: `forgeplan setup-skill` — устанавливает /forge skill в ~/.claude/skills/

## Dependencies

- cliclack = "0.5" (Rust port of @clack/prompts)
- console = "0.15" (terminal styling)

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-001 | Parent epic context | Active |
| NOTE-004 | FORGEPLAN-GUIDE documentation | Active |
