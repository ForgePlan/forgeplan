---
created: 2026-04-20
depth: deep
id: ADR-009
kind: adr
links:
- target: PROB-042
  relation: based_on
- target: ADR-008
  relation: informs
status: active
title: Forgeplan as orchestrator — Playbook Skill Agent Mapping Pack marketplace model
updated: 2026-04-20
---

# ADR-009: Forgeplan as orchestrator — Playbook/Skill/Agent/Mapping/Pack marketplace model

## Context

ADR-008 зафиксировал self-describing tools + agent-skills standard + brownfield-aware init. При проработке code-first brownfield use case (2026-04-20) выяснился фундаментальный pivot: **почти всё что мы собирались реализовать в forgeplan-core уже существует в adjacent плагинах экосистемы**:

| Capability | Plugin | Status |
|---|---|---|
| Structural architecture docs (C4 model, 4 levels, bottom-up code analysis) | `c4-architecture` (claude-code-workflows marketplace) | **installed** |
| Documentation generation from codebase (init/update/check/summarize modes, 8-phase pipeline) | `autoresearch` (Karpathy-derived) | в sources/, easy install |
| DDD bounded context / aggregate extraction | `agents-pro:ddd-domain-expert` | **installed** |
| SPARC specification workflow (requirements + AC + constraints) | `agents-sparc:specification` | **installed** |
| Adversarial 5-expert reasoning swarm + debate + converge | `autoresearch:reason` | в sources/ |
| Goal capture + scope + metric definition (4-question interview) | `autoresearch:plan` | в sources/ |

Существующий scope EPIC-006 (PRD-059..064) включал capability которые покрываются лучше:
- `forgeplan discover` vs c4-code + autoresearch:learn
- `forge-classify` LLM skill vs ddd-domain-expert
- `forge-dialogue` vs autoresearch:plan pattern
- `forge-migrator` orchestrator — сам по себе composition, подходящий паттерн

Получается: мы **изобретаем то что уже есть**, конкурируя с зрелыми инструментами. При этом **уникальная ценность forgeplan** — не генерация документов сама по себе, а **методологический слой поверх**: R_eff scoring, FPF trust calculus, graph traversal с typed links, lifecycle с transitions, ADI reasoning, evidence-backed decisions, markdown-primary storage (ADR-003). Эти примитивы **отсутствуют** в соседних плагинах.

Второй триггер — brownfield reference case: в параллельной сессии на aod-worker (105K LOC Go, 1180 commits) одинаково полезны три viewpoint'а: **Structural** (C4), **Behavioral** (DDD bounded contexts + use cases), **Historical** (inferred ADRs из git). Forgeplan сейчас не адресует ни один напрямую, но для каждого есть **специализированный** инструмент.

Третий триггер — symmetry greenfield: те же примитивы (Playbook, Skill, Agent, Mapping) работают для **capture vision → decompose → decide → scaffold → guardrail** flow. Не пересоздаём инфраструктуру для каждого use case, оркестрируем одну и ту же.

## Decision

**Forgeplan-core становится оркестратором** — знает **когда какой playbook запускать**, **кому делегировать каждый шаг**, и **как ингестить output в forge-граф**. Сам не генерирует документы (делегирует специализированным плагинам), но владеет lifecycle / scoring / graph / evidence binding.

Marketplace Forgeplan состоит из **4 примитивов + 1 composition unit**:

### Примитивы

1. **Playbook** (новое) — `.yaml` декларативная стратегия: триггеры, шаги, делегации, mappings, fallback hints. Unit of **strategy**.
2. **Skill** — `SKILL.md` per agent-skills standard. Unit of **capability**.
3. **Agent** — `AGENT.md` per agent-skills standard. Composes skills в role. Unit of **workflow**.
4. **Mapping** (новое) — `.yaml` описывающий как output внешнего плагина (C4-Documentation/, docs/, domain-model.md) → forge artifact (PRD/ADR/Epic/Note). Unit of **translation**.

### Composition unit

**Pack** — directory с manifest.yaml + playbooks/ + skills/ + agents/ + mappings/ + tests/. Unit of **distribution** через agent-skills standard marketplaces (agentskills.io + Claude Code plugin marketplace).

### Runtime responsibilities

Forgeplan-core получает **3 новые core capabilities**:

1. **Playbook runtime** (`forgeplan playbook {list|show|run|validate}`) — load YAML, resolve delegations, execute steps, capture outputs, handle missing-plugin fallback hints.
2. **Ingest engine** (`forgeplan ingest --mapping <file> --source <path>`) — apply mapping rules, generate forge artifact drafts, link к source files с line:col precision.
3. **Plugin detection + self-describing hints** — детектит installed plugins (`.claude/plugins/cache/`, `.agentskills/`, etc.), рекомендует playbooks based on project signals.

### Внешние плагины forgeplan **интегрирует через mappings**, не замещает:

- `c4-architecture` → `c4-to-forge.yaml` mapping
- `autoresearch` → `autoresearch-to-forge.yaml` mapping
- `agents-pro:ddd-domain-expert` → `ddd-to-forge.yaml` mapping
- `agents-sparc:specification` → `spec-to-forge.yaml` mapping
- Git history → `git-to-forge.yaml` mapping (требует новый skill `forge-history-miner` — gap filler)

## Alternatives Considered

| Option | Verdict | Why |
|---|---|---|
| **A. Status quo — forgeplan реализует всё сам** | Rejected | Изобретаем то что уже есть. C4, autoresearch, DDD agents — зрелые. Отстанем в качестве generation и не сможем поддерживать 7 harness adapters + domain expertise одновременно. |
| **B. Orchestrator — Playbook/Skill/Agent/Mapping/Pack** — **Chosen** | Chosen | Forgeplan делает то где unique: lifecycle + graph + scoring + evidence. Плагины делают то где лучше. Mappings — точка переиспользования. Packs распространяются через существующие marketplaces. |
| **C. Monorepo include всех плагинов** | Rejected | Copyright / license нарушения. Также — мы не контролируем версии upstream. |
| **D. Fork каждый плагин + customize под forge** | Rejected | 7 harness adapters × 4+ плагина = 28+ forks. Неподдерживаемо. |
| **E. Pure metadata layer (just links to external docs)** | Rejected | Теряем graph + scoring + evidence — основную ценность forgeplan. |

## Consequences

### Positive

- **Снижение scope EPIC-006** с 6 PRDs до 1-2 PRDs (docs-first migration playbook + mapping rules). Освобождает ~60% effort для core orchestration.
- **Composability** — один runtime обслуживает brownfield-docs, brownfield-code, greenfield, audit, release packs. No code duplication.
- **Leverage ecosystem** — пользуемся зрелыми инструментами, наша работа = thin integration layer.
- **Clear moat** — Forgeplan уникален как **lifecycle-aware graph over heterogeneous sources**. Ни C4, ни autoresearch, ни DDD-агенты этого не делают.
- **User experience symmetric** — одинаковый UX (`forgeplan playbook run X`) для brownfield-docs, brownfield-code, greenfield, audit.
- **Future-proof** — новые use cases = новые packs, без изменения core.
- **Marketplace native** — packs распространяются через существующие channels (agentskills.io + claude plugin marketplace).

### Negative (trade-offs)

- **Dependency chain** — forge рекомендует плагины которых может не быть. Mitigation: self-describing hints с install command (ADR-008 pattern).
- **Version skew** — плагины версионируются независимо, mapping может сломаться при upstream breaking change. Mitigation: per-mapping `compat_spec_version` + CI matrix + fallback hints.
- **Orchestration latency** — playbook = 7 steps, каждый вызов плагина 30s-5min. Total 30-45 min для full brownfield-code. Mitigation: parallelizable steps, progress reporting.
- **Learning curve** — user должен понимать playbook model. Mitigation: самые частые (brownfield/greenfield) — predefined playbooks, just run.
- **YAML fatigue** — 3 YAML формата (manifest + playbook + mapping). Mitigation: JSON schemas + validator + good defaults.

### Risks

- **Upstream plugin abandonment** — c4-architecture не maintained. Mitigation: mapping есть, user может fork. forgeplan-core не depends, только integrates.
- **Breaking changes в agent-skills standard** — indirectly affects us. Mitigation: изоляция per-pack, semver on mappings.
- **Scope creep packs** — соблазн включать всё-во-всё. Mitigation: 1 pack = 1 use case (5 core packs maximum: brownfield-docs, brownfield-code, greenfield, audit, release).

## Invariants

- **ADR-003 holds**: ингестированные форj-артефакты остаются markdown primary + LanceDB derived. External plugin outputs остаются в своих директориях (`C4-Documentation/`, `docs/`) — forge создаёт linked copies, не вывозит их к себе.
- **ADR-008 holds**: self-describing hints расширяются на playbook recommendations. Не ломает existing hint contract.
- **Plugin output read-only**: forge-ingest **не мутирует** `C4-Documentation/`, `docs/` — только читает и создаёт linked forge artifacts.
- **Idempotent ingestion**: повторный `forgeplan ingest` с тем же source → update существующих linked artifacts, не дубликаты.
- **Typed delegations**: каждая delegation в playbook — strict typed: `plugin:X / agent:X / skill:X / command:X / forgeplan_core:X`. No arbitrary shell без явного opt-in.
- **Opt-in orchestration**: `forgeplan playbook run` требует `--yes` или interactive confirm. Не запускается автоматически в init flow.

## Evidence Requirements

- **E1 — Mapping round-trip на real fixtures**: `c4-architecture` output на Forgeplan самом → `forgeplan ingest --from-c4` → validate: N forge artifacts created, каждый linked к C4 source file. **CL3 measurement**.
- **E2 — Playbook runtime на greenfield**: empty test-project → `forgeplan playbook run greenfield-kickoff` → verify: ADR-001 created, EPIC-001 scaffold, docs/ generated. CL3.
- **E3 — Brownfield-code E2E** на aod-worker fixture (105K LOC Go anonymized) → все 7 steps playbook → 30+ forge artifacts с correct kinds, все с `## Sources` section. CL3.
- **E4 — Plugin abandonment resilience**: uninstall c4-architecture → `forgeplan playbook run brownfield-code` → fails gracefully на step 1 с clear install hint. CL3.
- **E5 — Ingest idempotency**: `forgeplan ingest --from-c4 X` дважды подряд → второй запуск = update, не duplicate. Unit-testable.
- **E6 — Hallucination-proof**: каждый ingested artifact имеет `## Sources` с precise `file:line` ref. Если source удалён — artifact stale, не молча живёт. CL3.

## Valid Until

**Дата**: `2027-04-20` (12 месяцев).

**Обоснование TTL**: агрессивный timeline потому что agent-skills standard молод, marketplaces стабилизируются, packs паттерн для forgeplan — emerging.

**Refresh Triggers**:
- Major version bump agent-skills standard или Claude Code plugin marketplace spec
- Ecosystem shift (GitHub Copilot становится dominant + их свой plugin model)
- Data: >10 packs created community → refresh package policy / conventions
- Failure mode: >3 reports «playbook заблокировал — плагин ломается» → refresh delegation contract

## Pre-conditions (DoR)

- [x] ADR-008 active (self-describing foundation)
- [x] PROB-042 documenting orchestration gap
- [ ] Spike-1 выполнен: `/c4-architecture` на Forgeplan самом → CL3 evidence для E1
- [ ] Spike-2 выполнен: `/autoresearch:learn --mode init` → CL3 evidence для E1
- [ ] Manual mapping exercise done для c4-to-forge.yaml (минимум 1 mapping empirically validated)

## Post-conditions (DoD)

- [ ] Все 5 PRDs (PRD-065..069) activated с CL3 evidence
- [ ] EPIC-007 success criteria met (см. Epic)
- [ ] EPIC-006 scope narrowed to `brownfield-docs-pack` (один consumer of EPIC-007 runtime)
- [ ] brownfield-docs-pack, brownfield-code-pack, greenfield-pack minimum published
- [ ] `docs/operations/PLAYBOOK-AUTHORING.ru.md` — guide для pack authors
- [ ] `docs/schemas/playbook.schema.yaml` + `docs/schemas/mapping.schema.yaml` published
- [ ] 5+ mappings CL3-validated (c4/autoresearch/git/ddd/spec → forge)

## Admissibility

- **NOT**: forgeplan-core NOT реализует generation capabilities overlapping existing plugins (document generation, C4, bounded contexts). Only orchestrates.
- **NOT**: playbook runtime NOT trusts arbitrary shell output — каждый step declares `produces_at` structured path, парсинг через declared mapping.
- **NOT**: mapping YAML NOT embeds arbitrary code. Только declarative rules.
- **NOT**: ingest NOT writes outside `.forgeplan/` (ADR-003). Source files остаются untouched.
- **NOT**: playbook step NOT ignores fallback_hint — missing plugin → always emit install instruction, never silently skip.
- **NOT**: packs NOT required для base forgeplan workflow (init/new/validate/activate/score). Pack orchestration — opt-in advanced feature.

## Rollback Plan

**Triggers**:
- Playbook runtime instability — >3 reports failed runs с data loss
- Ingest mapping corrupting forge artifacts
- Fundamental incompatibility с agent-skills standard shifts

**Steps**:
1. Release v0.x.1 с `FORGEPLAN_PLAYBOOKS=0` env flag (disable playbook commands)
2. Ingest commands keep (read-only mappings OK как `forgeplan new` helpers)
3. EPIC-006 scope restored to original (discover/classify/migrate in-house)
4. Packs helper removed from skill-installer registry
5. User-installed packs → manual removal documented

**Blast Radius**: users adopted playbook workflow lose automation, fall back to `forgeplan new` + manual. MEDIUM — recoverable. Packs file на disk remain, re-enabled via flag.

## Weakest Link

**Agent-skills standard maturity** — та же concern что ADR-008 Weakest Link. Плагинная экосистема emerging, packs формат может drift. R_eff cap at CL2 до публикации formal agentskills.io spec.

**Plus**: plugin output stability. Если `/c4-architecture` меняет format между releases — наш `c4-to-forge.yaml` breaks. Mitigation: versioned mappings (compat range), CI matrix per mapping, fallback к manual ingest.

## Affected Files (planned)

| File | Baseline | Notes |
|------|----------|-------|
| `crates/forgeplan-core/src/playbook/` | — | **new module** — runtime, YAML schema, step executor |
| `crates/forgeplan-core/src/ingest/` | — | **new module** — mapping engine, transformation |
| `crates/forgeplan-core/src/plugins/` | — | **new module** — detection, hints |
| `crates/forgeplan-cli/src/commands/playbook.rs` | — | **new command** |
| `crates/forgeplan-cli/src/commands/ingest.rs` | — | **new command** |
| `docs/schemas/playbook.schema.yaml` | — | **new** |
| `docs/schemas/mapping.schema.yaml` | — | **new** |
| `docs/schemas/pack-manifest.schema.yaml` | — | **new** |
| `docs/operations/PLAYBOOK-AUTHORING.ru.md` | — | **new** |
| `marketplace/` (new directory) | — | canonical packs |

## AI Guidance

- Новая command в forgeplan → оцени, нужна ли она в ядре. Если capability покрывается external plugin — **написать mapping**, не реализовать самим.
- При pre-code phase каждого PRD из EPIC-007 — обязательно CL3 evidence spike. CL2 research недостаточно для deep decision blast radius.
- Playbook steps пишутся **декларативно** — delegate + produces_at + fallback. Императивная логика — в skills/agents.
- При added new pack — обязательно manifest.yaml + как минимум 1 integration test fixture.
- При added new mapping — обязательно versioned `compat_spec_version` + test fixture.
- При отсутствии plugin на user machine — self-describing hint MUST указать точное install command.

## Implementation Plan

### Phase 0: Shape (текущий)
- [x] PROB-042 captured
- [x] ADR-009 drafted
- [x] EPIC-007 created with children
- [x] PRD-065..069 stubs with shapes
- [ ] ADI reasoning (`forgeplan reason ADR-009`)
- [ ] EVID-081 research-level (CL2) для shape phase
- [ ] EPIC-006 scope narrowed
- [ ] ADR-009 + EPIC-007 activated

### Phase 1: Spikes (pre-code, DoR для PRDs)
- [ ] **Spike-1**: `/c4-architecture` на Forgeplan repo → CL3 evidence
- [ ] **Spike-2**: `/autoresearch:learn --mode init` → CL3 evidence
- [ ] **Spike-3**: manual ddd-domain-expert run → CL3 evidence
- [ ] EVID-081 upgrade to CL3

### Phase 2: Runtime foundation
- [ ] PRD-065: `forgeplan-core::playbook::` module + CLI
- [ ] PRD-066: `forgeplan-core::ingest::` + mapping YAML
- [ ] PRD-067: `forgeplan-core::plugins::` + hints extension

### Phase 3: Gap-fillers + orchestrator
- [ ] PRD-068: `forge-history-miner` skill
- [ ] PRD-069: `forge-orchestrator` agent + `forge-ingest` + `forge-scaffolder`

### Phase 4: Canonical packs
- [ ] `marketplace/brownfield-docs-pack/` (EPIC-006 refactored as consumer)
- [ ] `marketplace/brownfield-code-pack/` (new)
- [ ] `marketplace/greenfield-pack/` (new)

### Phase 5: Validation + rollout
- [ ] E2E: brownfield-code-pack на aod-worker fixture
- [ ] E2E: greenfield-pack на empty-repo fixture
- [ ] Publication к agent-skills marketplaces
- [ ] Docs published

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PROB-042 | Problem | based_on (documents gap this ADR addresses) |
| ADR-003 | ADR | informs (markdown source of truth invariant) |
| ADR-008 | ADR | informs (self-describing hints pattern расширяется) |
| EPIC-006 | Epic | informs (scope narrows — становится consumer EPIC-007) |
| EPIC-007 | Epic | drives (parent для 5 PRDs ниже) |
| PRD-065 | PRD | drives (playbook runtime) |
| PRD-066 | PRD | drives (ingest engine) |
| PRD-067 | PRD | drives (plugin detection + hints) |
| PRD-068 | PRD | drives (forge-history-miner skill) |
| PRD-069 | PRD | drives (orchestrator + ingest + scaffolder) |




