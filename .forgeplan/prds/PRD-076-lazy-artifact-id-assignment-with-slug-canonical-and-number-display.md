---
depth: standard
id: PRD-076
kind: prd
links:
- target: PROB-060
  relation: based_on
status: draft
title: Lazy artifact ID assignment with slug-canonical and number-display
---

---
id: PRD-076
title: "Lazy artifact ID assignment with slug-canonical and number-display"
status: Draft
author: explosivebit
created: 2026-05-06
updated: 2026-05-06
problem_ref: PROB-060
priority: P0
depth: deep
domain: general
projectType: cli_tool
stepsCompleted: []
---

# PRD-076: Lazy artifact ID assignment with slug-canonical and number-display

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/4   (  0%)  Foundation
Phase 1  ░░░░░░░░░░░░░░░░░░░░░░░░  0/6   (  0%)  Core schema + CLI
Phase 2  ░░░░░░░░░░░░░░░░░░░░░░░░  0/5   (  0%)  CI bot + MCP
Phase 3  ░░░░░░░░░░░░░░░░░░░░░░░░  0/4   (  0%)  Web + Skills
Phase 4  ░░░░░░░░░░░░░░░░░░░░░░░░  0/5   (  0%)  Migration + activation
─────────────────────────────────────────────────
TOTAL                               0/24  (  0%)
```

---

## Executive Summary

### Vision

Forgeplan создаёт артефакты с slug-каноническим идентификатором (`prd-auth-system`) и **отложенным** display-номером (`PRD-074`), который CI-бот атомарно присваивает на merge в dev. Это позволяет любому количеству людей и AI-агентов работать параллельно на разных ветках без ID-коллизий, сохраняя при этом привычную ментальную модель «PRD-074» в CLI/Web/Slack.

### Problem

**Что происходит сейчас**: `forgeplan new <kind>` использует counter `max(<kind>-NNN-*) + 1` (`crates/forgeplan-core/src/artifact/store.rs:23`). При параллельной работе на разных ветках или при multi-agent dispatch (PRD-057) два агента/разработчика независимо получают одинаковый ID. На merge — git add/add conflict, refs в commits указывают неоднозначно, semantic ref rot.

**Кому это плохо**:
- Команда из 2+ разработчиков на параллельных feature-branches
- Single dev запускающий `forgeplan_dispatch` с 3+ AI-агентами
- AI-агенты в multi-agent workflows которые видят неконсистентные IDs в результатах

**Impact**:
- 100% race-window между ветками без координации
- Ref rot в commit messages при reconciliation (commits immutable)
- Semantic ambiguity в search/ADI prompts при collision suffixes (PRD-074-a vs PRD-074)
- Multi-agent dispatch (PRD-057, v0.24.0) уже в production — проблема активна сейчас

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Solo dev | Один разработчик на одной ветке | Низкий impact: workspace lock защищает single-machine race |
| Team dev | 2-10 разработчиков на параллельных feature branches | ID-коллизии при merge, ref rot, время на reconciliation |
| Multi-agent operator | Запускает `forgeplan_dispatch` с 3-10 AI-агентами | Slug collisions у параллельных агентов, broken refs в их commits |
| AI-agent (consumer) | LLM работающий через MCP | Confusion от неконсистентных IDs, ambiguous search results, broken ADI references |
| Open-source contributor | Внешний contributor на форке | Не знает текущий next-id в upstream, конфликт на PR |

### Differentiators

- **Local-first preserved**: GitHub Actions `concurrency` group обеспечивает атомарность БЕЗ central server в нашей инфраструктуре — git остаётся source of truth
- **Backward compat**: 73 legacy артефакта продолжают работать с текущими номерами без изменений
- **Двухслойная identity**: slug каноничен, номер — display layer; UX «PRD-074» сохраняется
- **Multi-agent friendly**: slugs ортогональны при разных task titles → коллизий нет by construction

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Multi-agent dispatch без коллизий | Slug collisions per 100 dispatched tasks | unmeasured (race-window 100%) | 0 collisions при разных titles, ≤5% при одинаковых titles | Post-Phase-2 GA | EVID-B: 10×5-agent dispatch runs |
| SC-2 | CI-bot обеспечивает атомарность параллельных merges | Race conditions per stress-test | unmeasured (race-window 100%) | 0 (100% serialization on 10×concurrent-merge) | Post-Phase-0 | EVID-A: stress-test 10 simultaneous PR merges |
| SC-3 | AI-agent compliance с slug-refs | % коммитов с правильным slug в `Refs:` до merge | n/a | ≥ 95% | Post-Phase-4 | EVID-D: 50 reasoning-prompts benchmark |
| SC-4 | Backward compat для 73 legacy | Number of legacy artifacts с broken refs после migration | 0 | 0 | Phase-4 dry-run | EVID-C: migration dry-run script |
| SC-5 | Web rendering correctness | % виджетов корректно показывающих `?` marker для draft | 0% | 100% | Phase-3 | Visual regression suite ForgePlanWeb |
| SC-6 | Atomic merge serialization | Race conditions on parallel PR merges | unbounded | 0 (atomic via GitHub `concurrency`) | Phase-2 GA | Stress test: 5 simultaneous PRs |
| SC-7 | Time to first artifact creation | `forgeplan new` p95 latency | ~50ms | ≤ 200ms | Phase-1 | CLI benchmark |

---

## Product Scope

### MVP (In-Scope)

- Frontmatter schema: `slug`, `predicted_number`, `assigned_number` (см. SPEC-005)
- `forgeplan new <kind> "Title"` создаёт артефакт со slug, без assigned_number
- Pre-create check: warn если slug уже существует в origin/dev
- Derived id rendering: `id_display = assigned ?? predicted+"?"`
- `forgeplan get/search/list` принимают оба формата (slug и number)
- GitHub Actions workflow `assign-id.yml` с `concurrency: forgeplan-id-assign`
- Atomic next-number assignment на merge feat/* → dev
- Slug auto-suffix на slug-collision (двое выбрали одинаковый slug)
- `forgeplan reconcile-ids` команда (manual cleanup)
- MCP `forgeplan_new` response с slug, predicted_number, hint, _next_action
- ForgePlanWeb: derived id rendering, `?` marker для draft артефактов
- Migration script для 73 legacy артефактов
- Documentation: ID-ASSIGNMENT.ru.md, CLAUDE.md update, skill updates

### Out of Scope

- Переименование existing legacy artifacts (PRD-001..073 etc.) — frozen as-is
- Cross-workspace ID coordination (между разными forgeplan workspaces) — Non-Goal
- Number reservation API (заранее «забить» PRD-100 для будущего) — over-engineering
- ULID-based identity (Option B/C из FPF evaluation) — rejected, fallback only on EVID-A failure
- Distributed consensus protocols (Raft, Paxos) — нарушают local-first
- ID translation между разными conventions (`PRD-074` ↔ `prd-074` ↔ `prd_074`) — single canonical form

### Growth Vision

- Phase 5+: per-organization namespaces (`mycompany/PRD-074`) для multi-tenant deployments
- Phase 5+: Web UI для bulk reconciliation operations
- Phase 5+: ML-based slug suggestion (когда два title приводят к одинаковому slug, предложить разные)
- Phase 5+: ID stability metrics dashboard в Grafana

---

## User Journeys

### Journey 1: Team Dev — параллельная работа над разными темами

**Цель пользователя**: Создать PRD на своей ветке, не зная что коллега параллельно создаёт другой PRD.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `git checkout -b feat/auth` | — | branch from origin/dev |
| 2 | `forgeplan new prd "Auth System"` | Создан `prd-auth-system.md`, frontmatter `slug: prd-auth-system, predicted_number: 74, assigned_number: null`. Hint: `Use slug in commit Refs: until merged.` | Local create |
| 3 | Работа, коммиты с `Refs: prd-auth-system, FR-001..003` | — | slug в refs, не number |
| 4 | `gh pr create --base dev` | PR открыт | — |
| 5 | Merge PR | CI-бот: атомарно присваивает PRD-074, переименовывает файл, делает auto-commit `chore: assign PRD-074` | После merge — `?` уходит, остаётся `PRD-074` |

**Результат**: Коллега Bob параллельно создал `prd-rate-limiter` на своей ветке, тоже с `predicted_number: 74`. На его merge получает PRD-075 атомарно. Никаких коллизий, никакого ручного reconciliation.

### Journey 2: Multi-agent operator — `forgeplan_dispatch` с 5 агентами

**Цель пользователя**: Распределить 5 параллельных задач между AI-агентами, каждый создаёт по 1 PRD.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan_dispatch --agents 5 --plan plan.yaml` | Pre-allocates уникальные slugs для каждой задачи (`prd-auth`, `prd-rate-limit`, `prd-cache`, `prd-search`, `prd-billing`) | Slug allocation atomic в dispatcher |
| 2 | Агенты параллельно работают на 5 ветках | Каждый создаёт артефакт со своим pre-allocated slug | Zero collision by construction |
| 3 | Agents открывают PRs | 5 PRs в dev | — |
| 4 | Merge ordering: PR1 → PR2 → ... → PR5 | CI-бот сериализует через `concurrency`, присваивает PRD-074..078 | Atomic |

**Результат**: Все 5 артефактов созданы и активированы без ручной координации. Refs в коммитах каждого агента — slug-based, остаются валидными forever.

### Journey 3: AI-agent (consumer) — поиск артефакта через MCP

**Цель агента**: Найти артефакт по упоминанию в reasoning prompt.

| Шаг | Действие агента | Ответ системы | Заметки |
|-----|----------------|---------------|---------|
| 1 | Агент видит в prompt «как реализовано PRD-074» | — | — |
| 2 | `forgeplan_get(id="PRD-074")` | Возвращает артефакт с `slug: prd-auth-system, assigned_number: 74` | Number → canonical lookup |
| 3 | Агент видит в другом prompt «как relates к prd-auth-system» | — | — |
| 4 | `forgeplan_get(id="prd-auth-system")` | Возвращает тот же артефакт | Slug → canonical lookup |

**Результат**: Оба формата идентификатора работают transparently — агент не путается между slug и number.

### Journey 4: Solo Dev — создание hot-fix PROB

**Цель пользователя**: Срочно завести PROB по обнаруженному багу.

| Шаг | Действие | Ответ системы |
|-----|----------|---------------|
| 1 | `forgeplan new prob "API panic on null payload"` | `prob-api-panic-on-null-payload.md`, predicted_number: 61 |
| 2 | `forgeplan new evidence "Stack trace from prod"` | `evid-stack-trace-from-prod.md`, predicted_number: 113 |
| 3 | `forgeplan link prob-api-panic-on-null-payload --to evid-stack-trace-from-prod` | Линк создан по slug — будет валиден после merge |
| 4 | Merge PR — assigned PROB-061, EVID-113 | Все refs пересчитаны, frontmatter `assigned_number` выставлен |

**Результат**: Срочный фикс с full traceability, никакого «забил номер заранее, потом коллеги пересеклись».

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | Developer can create artifact via `forgeplan new <kind> "Title"`, getting back slug-based identity (`prd-auth-system`) and predicted_number, with assigned_number unset | J1, J2, J4 |
| FR-002 | Core | Must | System derives display id (`PRD-74?` или `PRD-074`) at render time based on `assigned_number ?? predicted_number+"?"` | J1, J3 |
| FR-003 | Core | Must | CI bot atomically assigns next free `assigned_number` per kind on merge to dev, using GitHub Actions concurrency group | J1, J2 |
| FR-004 | Core | Must | CI bot detects slug collision and auto-suffixes the later-merging PR's slug (`prd-auth-2`) | J1 |
| FR-005 | Core | Must | `forgeplan get/search/list` accept both slug and number identifiers, resolving to same canonical artifact | J3 |
| FR-006 | Core | Must | Pre-commit hook warns developer if local slug exists in origin/dev or open PRs | J1 |
| FR-007 | Multi-agent | Must | `forgeplan_dispatch` pre-allocates unique slugs across parallel agent tasks, eliminating slug collision by construction | J2 |
| FR-008 | UX | Must | CLI `forgeplan list` shows derived id with `?` marker for draft (unassigned) artifacts | J1, J4 |
| FR-009 | UX | Must | MCP `forgeplan_new` response includes `slug`, `predicted_number`, `assigned_number`, `hint`, and `_next_action` per PRD-071 hint protocol | J3 |
| FR-010 | UX | Must | ForgePlanWeb renders derived id in headers, NodeRef widgets, and graph nodes with `?` marker styling for drafts | J1 |
| FR-011 | Migration | Must | Migration script assigns slug + assigned_number to all 73 legacy artifacts without modifying their existing IDs or contents | J1 (post-cutoff) |
| FR-012 | Recovery | Should | Operator can run `forgeplan reconcile-ids` to detect and fix any post-merge ID coherence issues | J1 (recovery) |
| FR-013 | Validation | Must | Validator rejects manual `assigned_number` modifications in PR diffs (only CI bot can write this field) | J1 |
| FR-014 | Validation | Must | Slug regex validation: lowercase alphanumeric + hyphens, 3-80 chars, must start with kind prefix (per SPEC-005) | J1 |
| FR-015 | Search | Must | BGE-M3 search index keys on slug, assigned_number, predicted_number, AND title — search by any returns the artifact | J3 |
| FR-016 | Compatibility | Must | Existing `Refs: PRD-074` in commits and artifact bodies continue to resolve correctly post-migration | J3 (post-cutoff) |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | `forgeplan new` shall complete | ≤ 200ms p95 | Local execution, including `git fetch` | CLI bench |
| NFR-002 | Performance | CI bot ID assignment shall complete | ≤ 30s p95 | Per merge | GitHub Actions metrics |
| NFR-003 | Reliability | Atomic merge serialization | 100% | All concurrent PR merges | GitHub `concurrency` group |
| NFR-004 | Compatibility | Backward compat for 73 legacy IDs | 100% (zero broken refs) | All existing `Refs:` continue resolving | EVID-C migration dry-run |
| NFR-005 | Reliability | CI bot serialization | 100% atomic (0 race conditions) | 10 simultaneous PR merges via `concurrency` group | EVID-A stress-test |
| NFR-006 | Security | Reject PRs with manually-set `assigned_number` | 100% block rate | Pre-merge CI gate | CI workflow rule |
| NFR-007 | Observability | Migration script emits structured log | One JSON line per artifact migrated | Phase 4 execution | Log inspection |
| NFR-008 | Tooling | CLI/MCP/Web rendering tests | 100% pass | Pre-Phase-3 sign-off | Test suite |

---

## Acceptance Criteria

### AC-1: Two parallel branches create unrelated artifacts

```gherkin
Given Alice on branch feat/auth and Bob on branch feat/rate-limit
And both branched from same origin/dev (last PRD-073)
When Alice runs `forgeplan new prd "Auth System"` and Bob runs `forgeplan new prd "Rate Limit"`
Then Alice's artifact has slug `prd-auth-system`, predicted_number=74, assigned_number=null
And Bob's artifact has slug `prd-rate-limit`, predicted_number=74, assigned_number=null
And both can commit independently with `Refs: prd-auth-system` and `Refs: prd-rate-limit`
And on Alice merging first, her artifact gets PRD-074
And on Bob merging second, his artifact gets PRD-075 atomically
And no manual reconciliation is needed
```

### AC-2: Multi-agent dispatch — 5 parallel agents

```gherkin
Given operator runs `forgeplan_dispatch --agents 5 --plan plan.yaml`
When dispatcher pre-allocates unique slugs for each task
Then each agent receives a unique slug pre-assigned
And each agent creates artifact using assigned slug
And no slug collisions occur regardless of merge order
And final assigned_numbers are sequential (PRD-074..078) in merge order
```

### AC-3: Slug collision (rare) — auto-suffix on merge

```gherkin
Given Alice and Bob both ran `forgeplan new prd "Auth"` on different branches
And both got slug `prd-auth` and predicted_number=74
When Alice's PR merges first, getting PRD-074 with slug `prd-auth`
And Bob's PR enters merge queue
Then CI bot detects slug collision in Bob's PR
And auto-suffixes Bob's slug to `prd-auth-2`
And Bob's PR is renamed and gets PRD-075
And Bob is notified via PR comment about auto-suffix
And Bob's commit `Refs: prd-auth` are detected as ambiguous and flagged
```

### AC-4: Backward compatibility — legacy refs still work

```gherkin
Given existing artifact `prd-018-rfc-driven-architecture.md` with `assigned_number: 18`
And another artifact body contains `Related: PRD-018`
When user runs `forgeplan get PRD-018`
Then artifact is returned with slug `prd-rfc-driven-architecture` and assigned_number=18
And the `Related: PRD-018` reference resolves correctly
And no modifications are made to the legacy artifact
```

### AC-5: Web rendering — `?` marker for drafts

```gherkin
Given ForgePlanWeb is loaded with both draft and active artifacts
When viewing the artifact list
Then draft artifacts (assigned_number=null) show "PRD-74?" with dashed-border styling
And active artifacts show "PRD-074" without `?`
And clicking on either resolves to the correct artifact via /api/get/[id]
And graph node labels follow same convention
```

### AC-6: AI-agent uses slug in commit refs (compliance)

```gherkin
Given AI-agent is given task "create PRD for caching layer"
When agent runs `forgeplan new prd "Caching layer"`
And subsequently makes commits during implementation
Then agent's commit messages contain `Refs: prd-caching-layer` (slug)
And NOT `Refs: PRD-XXX?` or `Refs: PRD-074` (predicted/assumed number)
And on merge, CI bot assigns final number without breaking any refs
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| GitHub Actions concurrency groups | External | Available (since 2021) | GitHub |
| `forgeplan-core/src/artifact/frontmatter.rs` | Internal | Existing | core |
| `crates/forgeplan-mcp/src/tools/new.rs` | Internal | Existing | mcp |
| ForgePlanWeb (`template/src/widgets/...`) | External (other repo) | Active development | web team |
| `forgeplan_dispatch` (PRD-057) | Internal | Production v0.24.0 | core |
| Hint protocol (PRD-071) | Internal | Production v0.25.0 | mcp |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | `concurrency` primitive не сериализует параллельные merges ИЛИ multi-agent dispatch deadlock'ится | Low | High | Stress-test 10×concurrent-merge (EVID-A) в Phase 0. Reversal: alternative serialization (push hooks или maintainer-only assignment role) | impl |
| R-2 | AI-agents не используют slug в `Refs:` корректно (>5% non-compliance) | Medium | High | Explicit `hint:` в MCP responses. Section в CLAUDE.md. Skill update с примерами. EVID-D benchmark | mcp + docs |
| R-3 | Migration cutoff попадёт на open PRs со старой схемой | Medium | Medium | Grandfather rules для PRs открытых до cutoff. Choose cutoff на момент когда open PRs ≤ 3 | release |
| R-4 | Slug collision rate higher than expected (>5%) при одинаковых titles | Low | Low | Auto-suffix mechanism. ML-based slug suggestion в Growth Vision | impl |
| R-5 | ForgePlanWeb rendering breaks для legacy артефактов после migration | Low | Medium | Visual regression suite до и после migration. Feature flag `id_display_mode: legacy/hybrid/new` | web |
| R-6 | Race-condition в `forgeplan_dispatch` slug pre-allocation | Low | High | Atomic allocation в dispatcher через workspace lock. EVID-B benchmark | core |

---

## Timeline

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD Approved | 2026-05-13 | Requirements locked |
| SPEC Complete | 2026-05-15 | Frontmatter contract finalized |
| RFC Approved | 2026-05-17 | Migration plan agreed |
| ADR Activated | 2026-05-20 | Decision recorded with evidence |
| Phase 0 Complete | 2026-05-27 | EVID-A, EVID-C collected; ID-ASSIGNMENT.ru.md published |
| Phase 1 Complete | 2026-06-10 | Core schema + CLI |
| Phase 2 Complete | 2026-06-17 | CI bot + MCP |
| Phase 3 Complete | 2026-06-24 | Web + Skills |
| Phase 4 Complete (GA) | 2026-07-01 | Migration done, all EVID collected |

---

## Stakeholders

| Role | Name | Sign-off |
|------|------|----------|
| Product Owner | explosivebit | [ ] |
| Engineering Lead | explosivebit | [ ] |
| Web Team | ForgePlanWeb maintainers | [ ] |
| QA / Smoke Tests | (TBD) | [ ] |

---

## Affected Files

- `crates/forgeplan-core/src/artifact/store.rs` (next_id deprecated)
- `crates/forgeplan-core/src/artifact/frontmatter.rs` (new fields)
- `crates/forgeplan-core/src/artifact/types.rs` (slug validation, derived id)
- `crates/forgeplan-cli/src/commands/new.rs`
- `crates/forgeplan-cli/src/commands/reconcile.rs` (new)
- `crates/forgeplan-cli/src/commands/get.rs`, `list.rs`, `search.rs`
- `crates/forgeplan-mcp/src/tools/*.rs` (responses обновлены)
- `crates/forgeplan-core/src/playbook/dispatch/*.rs` (slug pre-allocation)
- `.github/workflows/assign-id.yml` (новый)
- `docs/methodology/ID-ASSIGNMENT.ru.md` (новый)
- `CLAUDE.md` (новая секция)
- `docs/operations/GIT-WORKFLOW.ru.md` (обновление)
- ForgePlanWeb: `template/src/widgets/artifact-panel/lib/markdown-export.ts`, `template/src/routes/api/get/[id]/+server.ts`, NodeRef widgets

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| PROB-060 | based_on | active (validated) |
| ADR-012 | decided_by | draft |
| SPEC-005 | implements | draft |
| RFC-009 | implementation_plan | draft |
| PRD-057 | informs (multi-agent dispatch context) | active |
| PRD-071 | informs (hint protocol contract) | active |
| ADR-003 | informs (markdown source of truth invariant) | active |



