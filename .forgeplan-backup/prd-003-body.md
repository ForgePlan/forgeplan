# PRD-003: Health Dashboard + Blind Spot Detection

## Problem

Чтобы понять состояние проекта нужно запустить 5 отдельных команд: status + stale + decay + validate + score по каждому артефакту. Это 5 вызовов с ручным анализом. Пользователь (и AI агент) не видит картину целиком — пробелы в evidence, просроченные данные, orphan артефакты скрыты за множеством отдельных команд.

## Goals

- Одна команда `forgeplan health` показывает полное здоровье проекта
- `forgeplan blindspots` — детальный анализ пробелов
- Compact mode для hooks/scripts (JSON < 500 tokens)
- MCP tools для AI agent integration

## Out of Scope

- Grafana / external dashboard integration
- Historical health tracking (trending)

## Target Users

Разработчики и AI агенты (Claude Code, Cursor) использующие Forgeplan.

## Functional Requirements

- [x] FR-001: `forgeplan health` — агрегированный dashboard (artifacts by kind/status, orphans, next actions)
- [x] FR-002: `forgeplan blindspots` — артефакты без evidence, orphans, missing links
- [x] FR-003: `forgeplan health --compact` — one-line output for hooks
- [x] FR-004: MCP tools: forgeplan_health, forgeplan_blindspots

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-001 | Parent epic | Active |
