---
depth: deep
id: EPIC-008
kind: epic
links:
- target: EPIC-007
  relation: based_on
- target: ADR-009
  relation: based_on
- target: ADR-008
  relation: informs
status: draft
title: Business Logic Extraction — Factum Intent Methodology
---

# EPIC-008: Business Logic Extraction — Factum/Intent Methodology

## Vision

Превратить Forgeplan из инструмента миграции документации в инструмент **обратной инженерии бизнес-логики**: из голого brownfield-кода без документации вывести structured business knowledge (глоссарий, use-cases, инварианты, Gherkin-сценарии, domain-model) с **explicit separation** между «что код делает» (Factum, 100% verifiable) и «почему так сделано» (Intent, hypothesis-driven, confidence-scored).

## Problem Space

Текущий Forgeplan + EPIC-007 runtime покрывают **document migration**: MADR/Obsidian/ad-hoc markdown → forge-граф. Но массовый brownfield-кейс — **код без документации вообще**. Autoresearch уже извлекает factum (code-map: file:line references), но это не self-contained бизнес-документация:

- Code-map становится dangling refs когда файлы двигаются/удаляются
- Нет ответа на «зачем это существует» (business rationale)
- Hypotheses смешаны с facts → документация untrustworthy
- Нет traceability: когда Domain Owner меняет intent, непонятно что обновлять

Design spec существует в `docs/brownfield-extraction-package/` (25 файлов, 12 bounded contexts, 12 skills, Factum/Intent methodology, ROADMAP 5 waves). Нужна integration с forgeplan как first-class extension.

## Goals

1. **Factum/Intent separation enforced** — validator блокирует artifacts где confidence-tag отсутствует на intent claims. Target: 100% intent paragraphs tagged (measurable via AST scan).
2. **6 new artifact kinds in forgeplan core** — `glossary`, `use-case`, `invariant`, `scenario`, `hypothesis`, `domain-model`. Target: `forgeplan new <kind>` для каждого работает, validation rules per kind.
3. **12-skill extraction pack published** — `forgeplan-extraction-pack` в peer marketplace repo. Target: `/extract-business-logic <domain>` запускает end-to-end extraction на sample brownfield.
4. **Hypothesis state machine** — first-class lifecycle (drafted → inferred → verified/refuted/parked → interview-resolved). Target: transitions enforced by validator + MCP `forgeplan_hypothesis_promote`.
5. **Confidence scoring per-assertion** — HTML-comment wrapping (`<!-- confidence: verified -->`) + parser + aggregation to artifact-level R_eff. Target: section-level confidence visible в projections.
6. **RAG-ready canonical output** — `/extract-business-logic --export rag` produces self-contained chunks (no file:line dangles). Target: validated via `forgeplan_reproducibility_check`.

## Non-Goals

- **NOT** замена EPIC-007 runtime — Extraction consumes playbook runtime (PRD-065) и ingest engine (PRD-066), не replicates их.
- **NOT** UI/dashboard для knowledge graph — только file-based markdown + JSON RAG export.
- **NOT** integration с external systems (Confluence, Notion) — pure file-based, git-versioned.
- **NOT** automation Domain Owner interview — генерируем packet, ингестим ответы, но cамo interviewing — human process.
- **NOT** замена ADR/PRD/Epic/Spec — 6 new kinds дополняют, не заменяют существующие.
- **NOT** breaking changes существующих kinds/workflows — всё additive.

## Target Users

1. **Brownfield solo maintainer** (primary) — один инженер, доставший legacy-проект без документации, хочет structured knowledge за недели, не годы.
2. **Team onboarding new hire** — новый инженер получает RAG-ready docs, глоссарий, Gherkin-сценарии, понимает систему за дни.
3. **Domain expert interviewer** — Product/Domain Owner получает clustered question-packets по unresolved hypotheses, одним sitting закрывает месяц research.
4. **Architecture auditor** — проверяет canonical DDL/SDL соответствует коду через `forgeplan_reproducibility_check`.

## Success Criteria

1. **Spec coverage**: все 25 файлов `docs/brownfield-extraction-package/` имеют соответствующие forge artifacts (6 kinds + 12 skills + orchestrator + integration specs). Measured: cross-ref audit.
2. **Factum coverage ≥ 80%** на референс-проекте TripSales: все public API endpoints, ORM-модели, guard-конструкции извлечены в use-case/invariant/domain-model.
3. **Intent confidence distribution** на TripSales: ≥ 40% verified/strong-inferred после Pass 3; unresolved items packaged как interview-packets.
4. **Hypothesis state machine enforced**: 0 PR merged с hypothesis в недопустимом transition (validator blocks).
5. **`/extract-business-logic orders` E2E**: на TripSales orders domain producit glossary (≥30 terms) + use-cases (≥10) + invariants (≥15) + scenarios (≥8) + domain-model, все validated.
6. **RAG export verified**: `forgeplan reproducibility check --domain orders` → PASS (canonical DDL parses via `psql --check`, SDL parses via GraphQL parser, scenarios parse via `@cucumber/gherkin`).
7. **Backward compat**: все 1405+ existing tests PASS, existing artifacts работают без изменений, users без adoption не видят поведения.
8. **Peer pack published**: `forgeplan-extraction-pack@ForgePlan-marketplace` installable, 12 skills detected by `forgeplan skill doctor`.

## Phases (from extraction-package ROADMAP, 5 waves)

### Wave 1 — Forgeplan Core Extensions (foundation)
Добавить в forgeplan-core: 6 new kinds + 10 new MCP tools + new relations (`defines`, `triggers`, `verifies`, `infers_from`, `resolved_by`, `parked_in`, `catalogs`, `emitted_by`, `causes`) + confidence HTML-wrapper parser + validation rules per kind. Plus foundation skills C1 (Ubiquitous Language) + C4 (Invariant Detector) в pack.

### Wave 2 — Use-cases + Causality
Skills C2 (Use-Case Miner) + C5 (Causal Linker). New autoresearch modes (`--mode use-case`, `--persona causality-analyst`).

### Wave 3 — Intent Generation
Skills C3 (Intent Inferrer) + C6 (Hypothesis Triangulator). Hypothesis state machine live. New autoresearch `--mode intent`.

### Wave 4 — Synthesis
Skills C7 (Interview Packager) + C8 (Scenario Writer) + C9 (KG Curator). Knowledge graph viz extension for `forgeplan_graph`.

### Wave 5 — Output + Orchestration
Skills C10 (Canonical Reproducer) + C11 (Reproducibility Validator) + C12 (RAG Packager). Meta-command `/extract-business-logic <domain>`. Orchestrates all 12 skills end-to-end.

## Children (PRDs, to be shaped)

| Type | ID | Title | Wave | Status |
|------|------|-------|--------|--------|
| PRD | PRD-070 (TBD) | 6 new artifact kinds + validation rules + templates | 1 | not-yet-shaped |
| PRD | PRD-071 (TBD) | Confidence scoring HTML-wrapper + aggregation + UI in projections | 1 | not-yet-shaped |
| PRD | PRD-072 (TBD) | 10 new MCP tools (hypothesis_*, coverage_business, interview_*, render_canonical, export_rag, reproducibility_check, contradictions, orphans) | 1 | not-yet-shaped |
| PRD | PRD-073 (TBD) | 12-skill extraction pack (forgeplan-extraction-pack) in peer marketplace | 2-5 | not-yet-shaped |
| PRD | PRD-074 (TBD) | `/extract-business-logic` orchestrator command | 5 | not-yet-shaped |
| PRD | PRD-075 (TBD) | Autoresearch integration (new modes glossary/use-case/invariant/canonical) | 2-5 | not-yet-shaped |

PRD IDs будут присвоены при создании. Shape сессия — отдельный PR после activate EPIC-008.

## Incorporated Scope (from prior feat-branch)

- **PRD-064-scope** (new kinds `kb/runbook/postmortem/retrospective/meeting` + new links `references`/`responds_to`/`caused_by`/`discusses`) — shape artifact PRD-064 жил на closed feat-branch `feat/prd-059-brownfield-pipeline` (PR #200 closed), никогда не merged в dev и не existed в main. Reindex 2026-04-21 удалил stale DB entry. Functional scope этих документационных kinds **subsumed** в EPIC-008 более формальными Factum/Intent kinds:
  - `kb` → `glossary` + `domain-model`
  - `postmortem` → `use-case` + `invariant` + `hypothesis` (каждый incident triangulates why-it-happened)
  - `runbook`/`retrospective`/`meeting` — остаются как optional future follow-up (если окажутся actually needed отдельно от extraction methodology)

## Dependencies

- **EPIC-007** (active): playbook runtime (PRD-065) + ingest engine (PRD-066) — foundation для 12-skill pack execution. EPIC-008 consume этот runtime.
- **ADR-009** (active): orchestrator model — EPIC-008 pack следует Pack marketplace structure.
- **ADR-008** (active): self-describing tools + agent-skills standard — EPIC-008 skills соответствуют SKILL.md формату.
- **autoresearch plugin** (external): extraction skills wire в `/autoresearch:learn`, `/autoresearch:reason`, `/autoresearch:predict`, `/autoresearch:scenario`.
- **design spec** `docs/brownfield-extraction-package/` — canonical source до merge в forge-артефакты. После Wave 1 complete package можно archive.

## Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Scope creep (8-10 session estimate per package ROADMAP) | High | High | Strict wave boundaries; each wave independently shippable; stop-at-wave-3 fallback (usable extraction без RAG export) |
| LLM hallucination в Intent tier (C3) | High | Medium | 3 diverse alternatives required; near-duplicate penalty; Domain Owner interview fallback |
| Confidence inflation (всё становится verified) | Medium | Medium | Quota: max 30% verified без Domain Owner input; validator enforces |
| Backward compat с existing workspaces | Low | Low | Additive-only changes; feature-flag new kinds initially |
| Autoresearch API changes | Low | Low | Wrap commands в skill abstractions; версия-пин в pack manifest |
| Domain Owner unavailable для TripSales E2E | High | Medium | Design accept partial completion; verified + inferred valuable даже без interview |
| PRD-064 scope gap (if runbook/meeting actually needed) | Low | Low | Add targeted PRD later if user feedback requests it — EPIC-008 doesn't block |
| Integration spec drift (package vs implemented) | Medium | Medium | Single source of truth rule; після Wave 1 — package archived, forge artifacts canonical |

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| EPIC-007 | Epic | based_on (runtime foundation) |
| ADR-009 | ADR | based_on (orchestrator model) |
| ADR-008 | ADR | informs (self-describing tools, skills standard) |
| EPIC-006 | Epic | informs (brownfield documentation migration complement; ingests scope of deleted PRD-064) |
| EVID-081 | Evidence | informs (orchestrator pivot research) |
| `docs/brownfield-extraction-package/` | External design spec | source_of_truth (до Wave 1 complete) |

## Implementation Order

Shape phase (не код):
1. Activate EPIC-008 (после review + ADI)
2. Shape 6 child PRDs (70-75) — один session per PRD или batch 3-at-once
3. Validate all PRDs
4. Reason (ADI) на PRD-070 (kinds/validation) — foundation

Code phase starts после Shape complete + EPIC-007 runtime shipped (v0.25+).

## Progress

```
Wave 1 (Foundation)        ░░░░░░░░░░░░░░░░░░░░░░░░  0/3  (  0%)
Wave 2 (Use-cases+Causal)  ░░░░░░░░░░░░░░░░░░░░░░░░  0/2  (  0%)
Wave 3 (Intent+Triangle)   ░░░░░░░░░░░░░░░░░░░░░░░░  0/2  (  0%)
Wave 4 (Synthesis)         ░░░░░░░░░░░░░░░░░░░░░░░░  0/3  (  0%)
Wave 5 (Output+Orch)       ░░░░░░░░░░░░░░░░░░░░░░░░  0/4  (  0%)
─────────────────────────────────────────────────────
TOTAL                                                0/14 (  0%)
```

## Acceptance Criteria для closing EPIC-008

- [ ] All 6 child PRDs shaped + validated + active
- [ ] Wave 1 Code complete: 6 kinds + 10 MCP tools + validation rules shipped в forgeplan v0.X
- [ ] Waves 2-5 Code complete: 12 skills shipped in forgeplan-extraction-pack
- [ ] E2E on TripSales orders domain: extraction produces complete knowledge graph, `/extract-business-logic orders` exits 0
- [ ] Reproducibility check PASS
- [ ] RAG export produces valid JSON bundle
- [ ] Docs published: forgeplan extensions guide, extraction-pack SKILL.md, orchestrator command docs
- [ ] Design spec `docs/brownfield-extraction-package/` archived (replaced by canonical forge artifacts)
- [ ] Hindsight memory entry про EPIC-008 completion




