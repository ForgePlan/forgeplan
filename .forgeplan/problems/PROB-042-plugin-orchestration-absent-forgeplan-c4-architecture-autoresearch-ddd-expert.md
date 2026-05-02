---
created: 2026-04-20
depth: tactical
id: PROB-042
kind: problem
status: active
title: Plugin orchestration absent — forgeplan реализует что уже есть в c4-architecture autoresearch ddd-expert плагинах
updated: 2026-04-20
---

# PROB-042: Plugin orchestration absent

## Problem Statement

При проработке code-first brownfield use case (разговор 2026-04-20 после первой iteration EPIC-006 Shape) выяснилось что значительная часть scope в PRD-059..064 дублирует capability уже существующих agent-skills плагинов: `c4-architecture` (structural docs), `autoresearch` (codebase docs generation), `agents-pro:ddd-domain-expert` (bounded contexts), `agents-sparc:specification` (requirements + AC). Forgeplan-core планируется реализовать своё LLM-classification skill (forge-classify), свой discover (в PRD-059), свой dialogue (в PRD-061) — конкурируя с зрелыми инструментами, при этом оставляя уникальную ценность Forgeplan (lifecycle + graph + scoring + evidence) не масштабированной к реальным workflows.

## Signal

Разведка 2026-04-20:
- `/Users/explosovebit/.claude/plugins/marketplaces/claude-code-workflows/plugins/c4-architecture/` уже установлен — 4 agents (c4-context, c4-container, c4-component, c4-code) + orchestrator command (`/c4-architecture`) делают bottom-up documentation кодовой базы. Outputs `C4-Documentation/*.md` (Mermaid + signatures + personas + user journeys).
- `sources/autoresearch/` — mature 8-phase pipeline Scout→Analyze→Map→Generate→Validate→Fix→Finalize→Log. Команды: plan, learn, debug, fix, security, ship, scenario, predict, reason. 3-harness compat (Claude Code, OpenCode, Codex).
- `agents-pro:ddd-domain-expert` installed, использован в 4-agent audit PROB-040 и нашёл 3 P0 DDD findings (aggregate ownership, status_map ACL, event-driven supersede).

Параллельная сессия aod-worker brownfield (105K LOC Go, 1180 commits): user хотел онбординг на legacy codebase. Нет инструмента который одновременно даёт **структуру** (c4) + **поведение** (ddd) + **историю решений** (git mining). Каждый viewpoint требует отдельного специализированного инструмента, которые forgeplan сам не реализует и не должен.

## Root Cause

При писании PRD-059..064 не было **инвентаризации** существующих agent-skills плагинов. Фокус был на «что forgeplan будет делать сам», а не «что forgeplan будет интегрировать». Это привело к scope который overlaps с open-source zero-sum (мы vs плагин автор конкурируем за один use case).

Архитектурная модель «Forgeplan = все-в-одном» неявно принята, но не подтверждена ADR. В ADR-008 (self-describing + agent-skills + brownfield-aware init) отсутствует положение о **делегации** — только о distribution (skills для других harness).

## Proposed Solution

Принять как ADR: **Forgeplan-core становится оркестратором**, не implementer-ом. Делегирует external plugins для специализированных capability (C4, docs generation, DDD, specs), ингестит их output в forge-граф через typed mappings. Unique value forgeplan: lifecycle + graph + scoring + evidence binding поверх heterogeneous sources.

См. ADR-009 для полной декомпозиции (Playbook/Skill/Agent/Mapping/Pack model) + EPIC-007 для implementation plan.

## Acceptance Criteria (for closing PROB-042)

- [ ] ADR-009 active
- [ ] EPIC-007 Phase 0 (Shape) complete
- [ ] EPIC-006 scope re-evaluated и narrowed (либо разделён, либо refactored как consumer EPIC-007)
- [ ] Первый canonical pack (brownfield-docs-pack) reframed as consumer of EPIC-007 runtime

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-009 | ADR | informs (proposes solution) |
| EPIC-007 | Epic | informs (implementation plan) |
| EPIC-006 | Epic | informs (scope re-evaluation triggered) |
| PROB-022 | Problem | informs (brownfield onboarding — parent) |
| PROB-040 | Problem | informs (shape audit findings — adjacent context) |

