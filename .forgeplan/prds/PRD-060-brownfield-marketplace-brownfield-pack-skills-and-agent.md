---
created: 2026-04-19
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
updated: 2026-04-19
---

# PRD-060: Brownfield — marketplace brownfield-pack skills and agent

## Problem

Core команды (PRD-058 discover/migrate) deterministic — они не классифицируют документы и не ведут диалог с пользователем. Для brownfield нужен LLM-powered layer: читает каждый файл, предлагает маппинг на forge kinds, задаёт вопросы пользователю по спорным кейсам, редактирует migration-plan.json. Этот layer — agent skill, распространяется через marketplace (agent-skills standard), не встраивается в forgeplan-core (LLM calls + dialogue не должны быть в deterministic Rust core).

## Goals

1. Canonical SKILL-пакет `brownfield-pack/` с 3 компонентами: `forge-classify`, `forge-dialogue`, `forge-migrator` (orchestrator agent).
2. Skills соответствуют agent-skills standard (BMAD skill-validator rules).
3. Skills редактируют `migration-plan.json` через документированный schema, не трогают forgeplan DB напрямую.
4. Package distributable через agentskills.io + Claude Code plugin marketplace.

## Non-Goals

- NOT пишет в LanceDB напрямую — только через forgeplan CLI/MCP commands
- NOT содержит harness-specific installer logic (это PRD-061)
- NOT делает мульти-LLM routing сам — использует `forgeplan` LLM config (inherit)

## Target Users

- **Brownfield adopter** — запускает agent/skill после `forgeplan discover`, получает guided migration
- **Skill author** — reference implementation для других forge-skills
- **Enterprise** — fork pack под свои conventions

## Success Criteria / Acceptance

- **AC-1**: `brownfield-pack/skills/forge-classify/SKILL.md` проходит BMAD skill-validator (14 deterministic rules).
- **AC-2**: skill description содержит «Use when»-clause (SKILL-06 rule).
- **AC-3**: `forge-classify` skill, дан `migration-plan.json` с unprocessed entries, возвращает plan с `predicted_kind` + `confidence` для каждого.
- **AC-4**: `forge-dialogue` skill задаёт user вопрос по entries с confidence < 0.7 или unknown kind, записывает решение в plan (`decision` field).
- **AC-5**: `forge-migrator` agent orchestrates: discover → classify → dialogue → migrate — через forgeplan commands.
- **AC-6**: Pack valid для agentskills.io standard (structure + manifest).
- **AC-7**: E2E test: 44-file Obsidian vault обрабатывается через agent end-to-end, все классифицированы, status preserved, plan saved.

## Functional Requirements

- **FR-1** Repo layout: `marketplace/brownfield-pack/` с `manifest.yaml` + `skills/{forge-classify,forge-dialogue}/SKILL.md` + `agents/forge-migrator/AGENT.md`.
- **FR-2** `forge-classify/SKILL.md` — LLM prompt для каждого entry в plan: analyze frontmatter + body, propose kind, confidence, reasoning.
- **FR-3** `forge-dialogue/SKILL.md` — для low-confidence entries задаёт пользователю multi-choice, обновляет plan.
- **FR-4** `forge-migrator/AGENT.md` — orchestrator: runs discover → classify → dialogue → migrate. Логи, resume support.
- **FR-5** Pack manifest: name, version, compat forgeplan version range, dependencies.
- **FR-6** References to core commands только через документированный surface (`forgeplan migrate --plan plan.json`).

## Implementation Plan

### Phase 1: Pack scaffolding
- [ ] **1.1** Repo layout + manifest.yaml schema
- [ ] **1.2** SKILL.md template + frontmatter conventions (agent-skills standard)

### Phase 2: Skills content
- [ ] **2.1** `forge-classify` SKILL.md с LLM prompt
- [ ] **2.2** `forge-dialogue` SKILL.md с interaction logic
- [ ] **2.3** `forge-migrator` AGENT.md с orchestration

### Phase 3: Validation + publication
- [ ] **3.1** BMAD skill-validator integration (14 deterministic rules)
- [ ] **3.2** E2E test на 44-file Obsidian fixture
- [ ] **3.3** Publish к agentskills.io + Claude Code plugin marketplace

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-008 | ADR | based_on |
| EPIC-006 | Epic | refines |
| PRD-058 | PRD | informs (skill operates through discover/migrate commands) |
| PRD-059 | PRD | informs (skill surfaced via self-description hints) |
| PRD-061 | PRD | informs (skill distributed by installer) |



