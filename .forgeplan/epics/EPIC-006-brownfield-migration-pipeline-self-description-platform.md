---
created: 2026-04-19
depth: deep
id: EPIC-006
kind: epic
links:
- target: ADR-008
  relation: based_on
- target: PROB-022
  relation: supersedes
status: active
title: Brownfield Migration Pipeline + Self-description Platform
updated: 2026-04-19
---

# EPIC-006: Brownfield Migration Pipeline + Self-description Platform

## Vision

Сделать Forgeplan first-class инструментом для brownfield-проектов любого масштаба — из Obsidian, MADR, ADR-tools, log4brains, ad-hoc `requirements/` и смешанных структур в структурированный forge-граф без потерь данных и без ручной работы. Одновременно перевернуть модель: **инструмент сам ведёт агента**, не наоборот. Поддержка не только Claude Code, но и 6+ других harness'ов (Cursor, Windsurf, Cline, Roo, Copilot, agentskills.io generic).

Telegram bug report 2026-04-19 с 33-ADR Obsidian vault'ом — reference scenario. Если этот vault импортируется без data loss с правильной классификацией, статусами, wikilinks, датами — цель достигнута.

## Problem

Текущий `forgeplan scan-import` (даже после PRD-058 hotfix) — жёсткий CLI-инструмент с regex и статическим маппингом. Любое отклонение от канона ломается: TOML frontmatter не парсится, нестандартные статусы (accepted, WIP, обсуждается) теряют семантику, Obsidian [[wikilinks]] не разрешаются в forge-граф, Epic-папки не маппятся на Epic-артефакты с nested PRDs, KB-статьи не имеют kind, terminal state `done` отсутствует.

Три арх-gap'а больше самого scan-import:
1. **Output opaque** — агент не знает следующего шага, должен помнить всё из CLAUDE.md.
2. **No cross-harness distribution** — скиллы живут в `.claude/skills/`, не видны из Cursor/Windsurf/etc.
3. **No brownfield-aware init** — пустой workspace не детектит legacy, не предлагает plan.

## Goals

1. **Brownfield onboarding < 5 минут**: `forgeplan init --from-brownfield` на реальном Obsidian vault → все документы в forge, без data loss, с правильными kinds, статусами, links.
2. **Cross-harness skill distribution**: `forgeplan skill install brownfield-pack` → skill доступен в detected harnesses (Claude Code / Cursor / Windsurf / Cline / Roo / Copilot / agentskills.io).
3. **Self-describing output**: каждая forgeplan-команда и MCP-tool эмитит next-step hint + required skill reference. Агент знает что делать, не читая CLAUDE.md.
4. **Context injection**: project conventions автоматически видны агенту через MCP tool descriptions — reliability over "hope agent remembers".
5. **Full semantic coverage**: KB, runbook, postmortem, retrospective, meeting — как первоклассные kinds с graph/vector leverage. State machine с `completed`/`archived` для brownfield-legacy.
6. **Bidirectional links**: supersede/deprecate atomically меняют обе стороны.

## Non-Goals

- **NOT** переписывать forgeplan CLI под OpenSpec action-based модель (Option D rejected в ADR-008)
- **NOT** заменять существующий Shape→Code→Evidence workflow — только расширяем brownfield entry
- **NOT** изобретать свой skill-формат — используем emerging agent-skills standard
- **NOT** support для non-markdown documentation форматов (AsciiDoc, rST, Jupyter) — только markdown + frontmatter в первой версии
- **NOT** auto-merge conflicts в skill-install — user confirm всегда

## Target Users

1. **Brownfield adopter** (primary) — команда с существующей документацией (ADR, PRD, KB, postmortems), хочет structured tool но не готова переписывать. Типичный сценарий: 30+ ADR в Obsidian, 5+ Epic-папок, 20+ KB, sprint-folders.
2. **Multi-harness user** (secondary) — использует Cursor/Windsurf/Copilot, не Claude Code. Хочет те же forge возможности.
3. **Solo maintainer greenfield** (existing) — уже работает в forge, получает self-describing hints бонусом — не платит цену.
4. **Enterprise scale** (future) — 500+ артефактов, cross-team, legacy 3+ лет. Benchmark для производительности.

## Success Criteria

1. **E2E migration test**: 44-файловый Obsidian vault (Telegram bug report reference) мигрирует через `forgeplan init --from-brownfield` с нулевой потерей данных. Проверяется автоматическим тестом.
2. **Status preservation**: `status: accepted` → `active`, `status: rejected` → `superseded`, `status: WIP` → warning+draft. Все 4 vocabularies (MADR, ADR-tools, log4brains, Obsidian custom).
3. **Wikilinks resolved**: все `[[ADR-007]]` в body source'ов → forge `references` links в graph.
4. **Dates preserved**: frontmatter `created: 2024-03-15` → artifact `created_at`, не now().
5. **Cross-harness verify**: на testbed с Claude Code + Cursor + Windsurf markers skill устанавливается в 3/3 корректных путей, detected via `forgeplan skill doctor`.
6. **Self-describing hints visible**: `forgeplan new prd "X"` stdout = текущий, stderr = hint ("next: fill MUST sections, validate; skill `forge-writer` в `.claude/skills/`").
7. **Context injection проверка**: MCP tool description для `forgeplan_new` содержит `project.context` из config.yaml при заполненном поле.
8. **Backward compat**: все текущие 1405 тестов проходят без изменений, существующие пользователи не видят изменённое поведение без `--from-brownfield` flag.
9. **Reindex no-loss**: после migration `forgeplan reindex` не удаляет ни одного импортированного артефакта (ADR-003 invariant).
10. **Docs published**: `docs/operations/BROWNFIELD-MIGRATION.ru.md` + `docs/schemas/agent-manifest.schema.json`.

## Phases

### Phase 0 — Shape (текущий)
Создать ADR-008, EPIC-006, 6 PRDs (A-F), все validate PASS, ADI для ADR, activate.

### Phase 1 — Core commands (PRD-A + PRD-B)
`forgeplan discover` + `migrate --plan --dry-run --apply --resolve-links`. Self-describing output convention. `agent-manifest` command. Context injection через `.forgeplan/config.yaml` → MCP tool descriptions.

### Phase 2 — Cross-harness distribution (PRD-C + PRD-D)
New crate `forgeplan-skill-installer` с 7 adapters. Canonical SKILL.md в `marketplace/brownfield-pack/`. Commands `forgeplan skill {list|doctor|install|uninstall|update}`. `forgeplan init --from-brownfield` detection wizard.

### Phase 3 — Semantic coverage (PRD-E + PRD-F)
State machine extension: `completed`/`archived` terminal states. Bidirectional `supersede`/`deprecate`. New kinds: `kb`/`runbook`/`postmortem`/`retrospective`/`meeting`. New link types: `references`/`responds_to`/`caused_by`/`discusses`.

### Phase 4 — Validation + rollout
E2E test 44-файлового Obsidian vault. Cross-harness CI matrix. `scan-import` deprecated в v0.25, removed в v0.27. Docs published.

## Children

| Artifact | Kind | Scope |
|----------|------|-------|
| PRD-A | PRD | `forgeplan discover` + `migrate --plan --dry-run --apply --resolve-links` — core commands |
| PRD-B | PRD | Self-description: stderr hints, `agent-manifest` command, `project.context` injection через MCP |
| PRD-C | PRD | Marketplace `brownfield-pack`: canonical SKILL.md, forge-classify skill, forge-dialogue skill, forge-migrator agent |
| PRD-D | PRD | `forgeplan init --from-brownfield` + new crate `forgeplan-skill-installer` с 7 harness adapters |
| PRD-E | PRD | State machine: `completed`/`archived` states, bidirectional `supersede`/`deprecate`, `forgeplan complete`/`archive` commands |
| PRD-F | PRD | New kinds: `kb`/`runbook`/`postmortem`/`retrospective`/`meeting`. New links: `references`/`responds_to`/`caused_by`/`discusses` |

IDs будут присвоены при создании (auto-increment).

## Dependencies

- PRD-058 (closed): scan-import ADR-003 foundation — без этого no projection base
- ADR-003 (active): Markdown = source of truth — MUST hold во всех child PRD
- ADR-008 (active): self-describing tools decision — драйвит B/C/D
- RFC-003 (active): Layered Architecture (traits) — `forgeplan-skill-installer` crate встраивается через trait pattern

## Progress

```
Phase 0 (Shape)     ░░░░░░░░░░░░░░░░░░░░░░░░  0/7  (  0%)
Phase 1 (PRD-A/B)   ░░░░░░░░░░░░░░░░░░░░░░░░  0/?  (  0%)
Phase 2 (PRD-C/D)   ░░░░░░░░░░░░░░░░░░░░░░░░  0/?  (  0%)
Phase 3 (PRD-E/F)   ░░░░░░░░░░░░░░░░░░░░░░░░  0/?  (  0%)
Phase 4 (Validate)  ░░░░░░░░░░░░░░░░░░░░░░░░  0/?  (  0%)
─────────────────────────────────────────────────────
TOTAL                                          0/?  (  0%)
```

## Risks

- **Scope creep**: Epic охватывает 6 PRDs, реально может разрастись. Mitigation: strict per-PRD scope boundaries, no cross-PRD feature leakage.
- **agentskills.io standard instability**: adapter rewrite risk. Mitigation: изолировать в одном crate (PRD-D).
- **Backward compat breakage**: опасность для существующих пользователей. Mitigation: все новые behaviors opt-in (flag или detection + confirm).
- **Rollout sequencing**: неправильный порядок PRD может блокировать cascade. Mitigation: явный order (см. Implementation Plan в ADR-008).

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-008 | ADR | drives (architectural decision) |
| PRD-058 | PRD | based_on (closed scan-import core bugs) |
| ADR-003 | ADR | informs (markdown = source of truth invariant) |
| RFC-003 | RFC | informs (layered architecture — new crate через trait pattern) |





