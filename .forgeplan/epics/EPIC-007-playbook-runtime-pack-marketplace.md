---
created: 2026-04-20
depth: deep
id: EPIC-007
kind: epic
links:
- target: ADR-009
  relation: based_on
- target: PROB-042
  relation: based_on
status: active
title: Playbook Runtime + Pack Marketplace
updated: 2026-04-20
---

# EPIC-007: Playbook Runtime + Pack Marketplace

## Vision

Forgeplan-core становится **оркестратором strategy**, не implementer. Маркетплейс packs (каждый — declarative workflow для конкретного use case: brownfield-docs, brownfield-code, greenfield, audit, release) использует forge-core runtime для: (a) плейбуковых шагов с делегацией external plugins, (b) ингестии их outputs в forge-граф с links к sources, (c) self-describing hints ведущих user к правильному playbook'у.

Результат: **один runtime обслуживает все use cases**, специализированные плагины (c4-architecture / autoresearch / ddd-expert / specification) делают что делают лучше, forgeplan связывает в consistent lifecycle-managed граф с R_eff scoring и evidence binding.

## Problem

Текущая модель forgeplan — «всё-в-одном»: discover + classify + migrate + skills + lifecycle + kinds — всё в core или marketplace brownfield-pack. Это приводит к дублированию зрелых open-source инструментов (PROB-042), раздуванию scope, и потере фокуса на уникальной ценности forgeplan.

Без orchestration layer:
- PRD-059 discover компонент конкурирует с c4-code + autoresearch:learn
- PRD-061 brownfield-pack skills конкурирует с ddd-domain-expert, specification agent
- Каждый новый use case (code-first brownfield, greenfield, audit automation) требует нового monolithic scope в EPIC
- Нет переиспользования: brownfield-docs workflow и code-first workflow имеют общие primitives (ingest, plugin delegation) но дублируются

## Goals

1. **Playbook as first-class** — declarative YAML format, forgeplan-core умеет load, validate, execute, report. Schema published, authorable юзерами.
2. **Ingest engine** — typed mapping YAML → forge artifacts с hallucination-proof links к source files (file:line precision). 5+ mappings (c4/autoresearch/git/ddd/spec → forge) CL3-validated.
3. **Plugin detection + hints расширение ADR-008** — forgeplan детектит installed/missing plugins и рекомендует playbooks с exact install commands.
4. **5 canonical packs** published: brownfield-docs (refactored из EPIC-006), brownfield-code, greenfield, audit, release. Каждый — consumer runtime.
5. **Scope narrowing EPIC-006** — из 6 PRDs до 1 PRD «brownfield-docs-pack» + 1 mapping (MADR/Obsidian/ADR-tools → forge). 60%+ effort released.

## Non-Goals

- NOT собственный LLM runtime (playbook делегирует к plugin LLM через Task tool / MCP client)
- NOT собственный document-generation engine (делегируем c4 / autoresearch / specification agents)
- NOT пытаемся быть «всё ещё better than C4» — мы OTHER layer (lifecycle над C4, не replacement)
- NOT поддержка non-YAML playbook formats (TOML/JSON) — единый YAML как в OpenSpec / autoresearch
- NOT support arbitrary shell в playbook steps без явного typed delegate

## Target Users

- **Brownfield adopter** — hits one command (`forgeplan init`), получает recommended playbook, runs, получает forge artifacts с linked sources
- **Greenfield kickstarter** — те же runtime, другой playbook (capture vision → decompose → stack ADR → scaffold)
- **Pack author** — пишет YAML manifest + playbook + mappings, публикует в marketplace
- **Existing forgeplan user** — не видит изменений в базовой работе (`forgeplan new|validate|activate|score`); orchestration — opt-in advanced

## Success Criteria

1. **E2E brownfield-code playbook** на anonymized aod-worker fixture (105K LOC Go) → 7 steps → 30+ forge artifacts с correct kinds, все с `## Sources` section.
2. **E2E greenfield-kickoff playbook** на empty repo → ADR-001 + EPIC-001 + 5 PRD stubs + docs/ scaffolded + skills installed в detected harness.
3. **E2E brownfield-docs playbook** (reframing EPIC-006) на 44-file Obsidian vault → 44 forge artifacts без data loss, status preserved, wikilinks resolved.
4. **5+ mappings CL3-validated**: c4-to-forge, autoresearch-to-forge, git-to-forge, ddd-to-forge, spec-to-forge.
5. **Plugin detection green**: workspace с тремя harness markers + установленными c4/autoresearch/ddd → `forgeplan playbook list` показывает applicable packs с readiness status.
6. **Ingest idempotency**: повторный `forgeplan ingest --from-c4 X` → update existing artifacts, no duplicates.
7. **Hallucination-proof**: каждый ingested artifact имеет `## Sources` section с file:line refs; `forgeplan doctor --sources` проверяет target files существуют.
8. **Backward compat**: все текущие 1405 тестов pass без изменений. Base workflow (`forgeplan new|validate|activate`) без изменений для пользователей не использующих packs.
9. **Docs**: `PLAYBOOK-AUTHORING.ru.md` + `INGEST-MAPPINGS.ru.md` + 3 schemas (playbook, mapping, manifest) published.
10. **Scope narrowing EPIC-006 proven**: measurement — lines in PRD-059..064 before pivot vs after re-frame. Target: >50% reduction.

## Phases

### Phase 0 — Shape (текущий)
Создать ADR-009, PROB-042, EPIC-007, PRD-065..069 stubs с clear scope + dependencies. ADI на ADR-009 (3+ hypotheses). EVID-081 research-level (CL2). EPIC-006 scope narrowed в этом же commit set. ADR-009 + EPIC-007 activated. Shape PR готов к review.

### Phase 1 — Spikes (pre-code, DoR blocker для PRDs)
- Spike-1: `/c4-architecture` agents на Forgeplan repo → получить C4-Documentation/ → manual mapping exercise → CL3 evidence что concept works.
- Spike-2: `/autoresearch:learn --mode init` на Forgeplan → docs/ → compare with existing → CL3 evidence.
- Spike-3: manual ddd-domain-expert run on Forgeplan → domain-model.md → CL3 evidence.
- EVID-081 upgrade от CL2 research до CL3 measurement после spikes.

### Phase 2 — Runtime foundation (PRD-065 + PRD-066 + PRD-067)
`forgeplan-core::playbook::` module (YAML schema + executor + step dispatch). `forgeplan-core::ingest::` (mapping engine). `forgeplan-core::plugins::` (detection + hints). CLI surface: `forgeplan playbook {list|show|run|validate}`, `forgeplan ingest --mapping --source`.

### Phase 3 — Gap-fillers + orchestrator (PRD-068 + PRD-069)
`forge-history-miner` skill (git log + blame → inferred ADR — gap filler, нет существующего plugin для этого). `forge-orchestrator` agent (universal playbook executor via agent-skills). `forge-ingest` skill (transformation engine exposed as skill). `forge-scaffolder` agent (greenfield bootstrap).

### Phase 4 — Canonical packs
- `marketplace/brownfield-docs-pack/` — EPIC-006 refactored as consumer (1-2 PRD scope)
- `marketplace/brownfield-code-pack/` — playbook + 3 mappings (c4, autoresearch, git)
- `marketplace/greenfield-pack/` — playbook + 2 mappings (vision, decomposition)
- (future) `audit-pack`, `release-pack`

### Phase 5 — Validation + publication
- E2E tests: все 3 packs
- CI matrix per pack per harness
- Publication к agentskills.io + Claude Code plugin marketplace
- Docs published: PLAYBOOK-AUTHORING.ru.md + INGEST-MAPPINGS.ru.md + schemas

## Children

| Artifact | Kind | Scope |
|----------|------|-------|
| PRD-065 | PRD | Playbook YAML schema + runtime executor (`forgeplan playbook ...`) |
| PRD-066 | PRD | Ingest engine + mapping YAML format + 5 core mappings |
| PRD-067 | PRD | Plugin detection + self-describing hints playbook recommendations |
| PRD-068 | PRD | forge-history-miner skill (git → inferred ADR — gap filler) |
| PRD-069 | PRD | forge-orchestrator agent + forge-ingest skill + forge-scaffolder agent |

## Dependencies

- ADR-009 (active): decision to orchestrate, not implement
- ADR-008 (active): self-describing tools foundation (расширяется в PRD-067)
- ADR-003 (active): markdown = source of truth (ingest invariant)
- EPIC-006 (active, scope narrowed): становится consumer EPIC-007 runtime для brownfield-docs playbook
- External plugins: c4-architecture, autoresearch, agents-pro, agents-sparc (все installed / installable)

## Progress

```
Phase 0 (Shape)           ░░░░░░░░░░░░░░░░░░░░░░░░  0/5  (  0%)
Phase 1 (Spikes)          ░░░░░░░░░░░░░░░░░░░░░░░░  0/3  (  0%)
Phase 2 (Runtime)         ░░░░░░░░░░░░░░░░░░░░░░░░  0/3  (  0%)
Phase 3 (Gap-fillers)     ░░░░░░░░░░░░░░░░░░░░░░░░  0/2  (  0%)
Phase 4 (Packs)           ░░░░░░░░░░░░░░░░░░░░░░░░  0/3  (  0%)
Phase 5 (Validation)      ░░░░░░░░░░░░░░░░░░░░░░░░  0/4  (  0%)
─────────────────────────────────────────────────
TOTAL                                                 0/20 (  0%)
```

## Risks

- **Ecosystem dependency** — крупный risk. c4-architecture, autoresearch могут не поддерживаться. Mitigation: mappings versioned, graceful fallback, self-describing hints.
- **Pack authoring adoption** — external contributors могут не писать packs. Mitigation: 5 canonical packs от нас как reference, detailed docs, schemas с auto-complete.
- **Scope overlap с EPIC-006** — risk диверсии. Mitigation: в рамках этого же commit narrow EPIC-006 scope до brownfield-docs-pack consumer.
- **YAML schema evolution** — breaking changes mappings format. Mitigation: semver per mapping, version field в YAML.
- **Plugin output format drift** — c4 меняет format между releases. Mitigation: per-mapping `compat_spec_version`, CI matrix per plugin version.

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| ADR-009 | ADR | drives (architectural decision) |
| PROB-042 | Problem | based_on (gap this Epic addresses) |
| ADR-008 | ADR | informs (self-describing расширяется в PRD-067) |
| ADR-003 | ADR | informs (ingest invariant) |
| EPIC-006 | Epic | informs (scope narrowed, refactored as consumer) |




