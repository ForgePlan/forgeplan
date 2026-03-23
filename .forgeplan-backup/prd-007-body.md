# PRD-007: Artifact Lifecycle Workflow

## Problem

PROB-003: все артефакты застревают в draft навсегда. Статусы — мёртвые метаданные без enforcement. Нет формализованного процесса принятия решения о готовности артефакта. Пользователь создаёт PRD, заполняет Summary и FR, но никогда не переводит в Active потому что нет процедуры review. В результате все 18 артефактов проекта остаются в Draft — включая полностью реализованные.

## Goals

- Формализованный lifecycle: Draft → Review → Active → Superseded/Deprecated
- Validation gates на каждом переходе (MUST rules must pass)
- Автоматические предупреждения о build-on-draft и supersede chain
- CLI + MCP commands для всех lifecycle операций

## Out of Scope

- Approval by multiple reviewers (single-user tool)
- Git hooks for status enforcement
- Notifications (no notification system)

## Target Users

Разработчики и архитекторы использующие Forgeplan для документирования решений.

## Functional Requirements

- [x] FR-001: `forgeplan review <id>` — запускает validation, показывает checklist, предлагает activate
- [x] FR-002: `forgeplan activate <id>` — draft → active с validation gate (MUST rules пройдены)
- [x] FR-003: `forgeplan supersede <id> --by <new-id>` — active → superseded + auto-link
- [x] FR-004: `forgeplan deprecate <id> --reason "..."` — active → deprecated
- [x] FR-005: Build-on-draft warning — validate предупреждает если RFC based_on draft PRD
- [x] FR-006: Supersede chain warnings — при supersede все зависимые получают notification
- [x] FR-007: MCP tools для всех lifecycle commands
- [x] FR-008: `forgeplan health` показывает draft/active/superseded ratio

## Lifecycle State Machine

```
Draft ──review──→ Draft (if validation fails)
Draft ──activate──→ Active (if validation passes)
Active ──supersede──→ Superseded (link to replacement)
Active ──deprecate──→ Deprecated (with reason)
Superseded ──→ (terminal)
Deprecated ──→ Active (un-deprecate allowed)
```

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-001 | Parent epic | Active |
| PROB-003 | Motivating problem | Draft |
