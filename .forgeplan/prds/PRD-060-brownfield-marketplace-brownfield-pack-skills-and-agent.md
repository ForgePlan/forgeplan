---
depth: standard
id: PRD-060
kind: prd
links:
- target: EPIC-006
  relation: refines
- target: ADR-008
  relation: based_on
status: draft
title: Brownfield — marketplace brownfield-pack skills and agent
---

# PRD-060: Brownfield — self-description + agent-manifest + context injection

## Problem

Forgeplan CLI и MCP-tools не говорят агенту что делать дальше. Агент должен помнить весь workflow из CLAUDE.md. При brownfield это особенно плохо — агент не знает что есть discover/classify/dialogue phases, где лежит skill, где взять инструкции. Project conventions (cargo fmt, tests before commit) агент вспоминает по памяти, не всегда. Результат: inconsistent behavior, повторяющиеся ошибки.

## Goals

1. Каждая CLI-команда эмитит structured hint в stderr: next-step + required skill + install command.
2. Каждый MCP-tool возвращает hint в отдельном JSON-поле response.
3. `forgeplan agent-manifest` — JSON с рекомендациями per-operation, versioned schema.
4. `.forgeplan/config.yaml` расширен `project.context` + `project.rules_per_kind` — автоматически инжектится в MCP tool descriptions.

## Non-Goals

- NOT меняет exit codes (0 success, hints informational)
- NOT добавляет интерактивность в CLI (hints != prompts)
- NOT заменяет CLAUDE.md как source of project conventions — дублирует только hot path для tool usage

## Target Users

- **All forgeplan users** — hints видны всегда (disable через `FORGEPLAN_HINTS=0`)
- **Brownfield adopter** — hints ведут через discover → migrate → validate flow
- **MCP harness user** — `project.context` автоматически виден с каждым tool-call

## Success Criteria / Acceptance

- **AC-1**: `forgeplan new prd "X"` stdout не изменился, stderr содержит блок `⚠ next: fill MUST sections; skill 'forge-writer' available`.
- **AC-2**: MCP tool response имеет поле `_hints: { next: "...", required_skill: "...", install: "..." }`.
- **AC-3**: `forgeplan agent-manifest` → valid JSON per `docs/schemas/agent-manifest.schema.json`. Schema versioned (semver).
- **AC-4**: При заполненном `project.context` в config, MCP описание `forgeplan_new` содержит блок с этим context.
- **AC-5**: `FORGEPLAN_HINTS=0` отключает stderr hints полностью. Backward compat для CI.
- **AC-6**: `isatty(stderr) == false` — hints не печатаются (не засоряют пайпы).
- **AC-7**: Все существующие 1405 тестов проходят без изменений.

## Functional Requirements

- **FR-1** Hint convention module в forgeplan-core: `Hint { next, required_skill, install, docs_ref }` — struct + serializer.
- **FR-2** Per-command hint registry — каждая CLI-команда привязана к Hint через declarative mapping.
- **FR-3** stderr hint renderer — respects `FORGEPLAN_HINTS`, `isatty`, локализация (EN/RU optional).
- **FR-4** MCP tool response enrichment — добавляет `_hints` поле в каждый response. Не ломает существующие schemas.
- **FR-5** `forgeplan agent-manifest` command — читает config + registry, возвращает JSON.
- **FR-6** agent-manifest schema (`docs/schemas/agent-manifest.schema.json`) — formal JSON schema с versioning.
- **FR-7** Config поля: `project.context: string (max 8 KB)`, `project.rules_per_kind: map<kind, string[]>`. Optional.
- **FR-8** MCP tool description injection — при старте MCP server читает `project.context`, dynamically appends в каждый tool description.
- **FR-9** Fallback: если config отсутствует или поле пустое — no injection, default descriptions.

## Implementation Plan

### Phase 1: Hint infrastructure
- [ ] **1.1** Hint struct + registry в forgeplan-core
- [ ] **1.2** stderr renderer с env flags and TTY check
- [ ] **1.3** Подключение к 5 key commands (new, validate, activate, migrate, discover)

### Phase 2: MCP enrichment
- [ ] **2.1** Tool response `_hints` поле
- [ ] **2.2** Context injection в tool descriptions at MCP server start
- [ ] **2.3** Config schema для `project.context` + `project.rules_per_kind`

### Phase 3: agent-manifest
- [ ] **3.1** `forgeplan agent-manifest` command
- [ ] **3.2** `docs/schemas/agent-manifest.schema.json`
- [ ] **3.3** Versioning policy doc

### Phase 4: Tests + docs
- [ ] **4.1** Unit tests для hint rendering, env flags, TTY
- [ ] **4.2** E2E test: `project.context` виден в MCP tool description
- [ ] **4.3** Docs: `docs/operations/SELF-DESCRIBING-HINTS.ru.md`

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-008 | ADR | based_on |
| EPIC-006 | Epic | refines |
| PRD-059 | PRD | informs (hints эмитятся из discover/migrate commands) |




