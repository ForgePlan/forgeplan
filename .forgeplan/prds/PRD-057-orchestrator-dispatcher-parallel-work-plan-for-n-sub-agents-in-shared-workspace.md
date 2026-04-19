---
depth: standard
id: PRD-057
kind: prd
links:
- target: EPIC-005
  relation: based_on
status: draft
title: Orchestrator dispatcher — parallel work plan for N sub-agents in shared workspace
---

---
id: PRD-057
title: "Orchestrator dispatcher — parallel work plan for N sub-agents in shared workspace"
status: Draft
author: gogocat
created: 2026-04-19
updated: 2026-04-19
priority: P0
depth: standard
domain: general
projectType: cli_tool
epic: EPIC-005
stepsCompleted: []
---

# PRD-057: Orchestrator dispatcher — parallel work plan for N sub-agents in shared workspace

## Progress

```
Inc 1 (lock)          ████████████████████████  3/3   (100%)  ✅ v0.23.1 merged
Inc 2 (identity)      ████████████████████████  3/3   (100%)  ✅ FR-009 + AC-5
Inc 3 (claims)        ████████████████████████  4/4   (100%)  ✅ FR-004..006,014 + AC-2,3
Inc 4 (dispatch)      ░░░░░░░░░░░░░░░░░░░░░░░░  0/4   (  0%)
─────────────────────────────────────────────────────────
TOTAL                                           10/14  ( 71%)
```

**Inc 2 delivered (2026-04-19)**:
- `AgentIdentity` struct in `forgeplan-core::artifact::identity`
- Unknown-frontmatter preservation via `KNOWN_FM_KEYS` + `filter_preserved` in `projection`
- `projection::stamp_agent_identity` helper
- `ForgeplanServer::stamp_identity_best_effort` wired into `forgeplan_new` + `forgeplan_update`
- `call_tool` wrapper captures `peer.peer_info()` → cached in server + logged to activity JSONL
- 13 new tests (6 identity + 4 projection preservation + 4 MCP stamp wiring), 1314 total

**Inc 3 delivered (2026-04-19)**:
- `forgeplan-core::claim` module: `Claim` struct, `ClaimStore`, `ClaimError`
- TTL bounds: 1 min ≤ `ttl` ≤ 24 h, default 30 min
- Same-agent calls renew; expired claims transparently taken over (AC-3)
- `forgeplan_claim`, `forgeplan_release`, `forgeplan_release --force`, `forgeplan_claims` MCP tools
- Agent resolution: explicit `agent` param > cached MCP `clientInfo` > hinted error
- All writes serialized via `workspace_lock` (Inc 1 pattern extended)
- 24 new tests (17 Core ClaimStore + 7 MCP wiring), 1338 total

**R2 mid-sprint audit hotfix (2026-04-19, 3 agents):**
- HIGH×1: path traversal in `ClaimStore::path_for` → `validate_id` guard at every entry point
- HIGH×2: `release` empty-agent bypass + `forgeplan_release` fragile force path → agent check before FS, deterministic resolution order
- MED×4: atomic `tempfile+rename` writes (`claim::atomic_write` + `projection::atomic_markdown_write`), Unicode/control-char rejection in `AgentIdentity::new`, `list_active_with_stats` surfaces malformed-file count, `forgeplan_claims` no longer holds workspace lock (read-only)
- MED×1: `list_active_map` for O(1) dispatcher lookups (Inc 4 forward-compat)
- LOW×1: TTL clamp at MCP boundary matches advertised schema
- +14 new regression tests (9 claim hardening + 5 identity hardening), 1352 total
- Defer to v0.25+: shared `kv_yaml` abstraction extraction, HTTP/SSE identity refactor, ADR for claim/phase separation

---

## Executive Summary

### Vision

Когда orchestrator (человек или AI-агент) запускает 2–5 sub-agents в shared
workspace Forgeplan'а, один MCP-вызов `forgeplan_dispatch --agents N` даёт
**готовый план работы**: какие артефакты можно делать параллельно, какие
сериально, кто над чем работает, с учётом зависимостей и overlap'а по коду.
Это превращает Forgeplan из каталога артефактов в **активный диспетчер**
для многоагентной разработки.

### Problem

Сейчас Forgeplan отлично хранит артефакты (PRD/RFC/ADR/Evidence) и их
зависимости, но НЕ отвечает на главный вопрос multi-agent оркестратора:

> «У меня 3 агента готовы. Кому что дать чтобы работали параллельно и
> не наступали друг на друга?»

Ответ сейчас — вручную: orchestrator читает `forgeplan_graph`, применяет
`forgeplan_blocked`, смотрит `forgeplan_list --status draft`, анализирует
`affected_files` в каждом PRD, держит всё в голове. Масштаб 2-5 агентов
ломает человеческую способность это tracking'овать — начинается:
- Двойная работа (2 агента берут один PRD)
- File conflict (2 агента меняют один crate → race / merge hell)
- Serial wasted (работа A ждёт B хотя могла бы идти параллельно с C)
- Забытый blocker (PRD активирован пока его deps not ready)

**Impact** — из собственного опыта этой session:
- В ходе работы над PRD-055 увеличенно 3 incrementа: пришлось делать
  serial (receipt → wrap → restore) один агентом. С dispatcher можно
  было бы разделить на 3 агента.
- Post-ship audit v0.22.1 + v0.23.0 Rounds 1+2: 4 agents работали
  параллельно через `Agent` tool — но оркестрация была у меня в голове,
  без Forgeplan'а как brain.
- Существующие `graph`/`order`/`blocked` покрывают 80% нужной логики,
  но нет agent-facing tool который собирает их в единый план.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Orchestrator (primary) | Человек / AI-агент запускающий sub-agents через Task tool или MCP | Не видит за один вызов "что можно параллелить с N агентами безопасно" |
| Sub-agent (secondary) | Отдельная Claude Code session / Task subagent работающая над одним артефактом | Не знает что кто-то уже работает над соседним PRD → конфликт |
| Project auditor (tertiary) | Человек пересматривающий историю | Сейчас невозможно узнать "какой агент принял какое решение" без ручного git blame |

### Differentiators

- **Использует существующие primitives** — `forgeplan_graph` DAG, `forgeplan_order`
  topological sort, `forgeplan_blocked` detector, `affected_files` frontmatter.
  ~80% логики уже реализовано, новый tool собирает их в actionable план.
- **Git-native claim protocol** — не нужна отдельная infrastructure.
  `forgeplan_claim` пишет маркер в `.forgeplan/claims/<id>.yaml` +
  (опционально) draft PR title `[WIP claim PRD-X by <agent>]`.
  Другие агенты видят claim через MCP или git.
- **File-overlap detection** — из `affected_files` frontmatter каждого артефакта,
  dispatcher вычисляет Jaccard similarity между file sets. Артефакты с
  overlap > threshold маркируются serial-only.
- **Skill matching** — каждый артефакт имеет `domain: frontend/backend/api/infra`
  в frontmatter; оркестратор передаёт skills каждого агента; dispatcher
  матчит.
- **Advisory, не enforced** — работает параллельно с `forgeplan_phase`
  (PRD-056): dispatcher суґерирует план, claims — soft signal, оркестратор
  может override. Full enforcement в v0.25+.

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | `forgeplan_dispatch --agents N` возвращает работоспособный план в < 500 мс | p95 latency | — | < 500ms at 100 artifacts | v0.24.0 ship | bench test |
| SC-2 | 0 false-positive "parallel" между артефактами у которых пересекаются affected_files | false-positive rate | — | 0% | v0.24.0 | integration test |
| SC-3 | Orchestrator видит claim за < 100мс после того как другой agent заявил work | propagation latency | — | < 100ms | v0.24.0 | filesystem notify / poll test |
| SC-4 | Нет breaking changes в существующих tools | breaking count | N/A | 0 | CI | cargo test |
| SC-5 | Dogfood: 3 sub-agents работают параллельно без конфликта в одной сессии | conflict count | — | 0 | post-ship | manual E2E |

---

## Product Scope

### MVP (In-Scope)

**MCP tools**:
- `forgeplan_dispatch --agents N [--skills csv] [--kind filter] [--epic ID]`:
  возвращает план с bucket'ами per-agent + serial queue + reasoning
- `forgeplan_claim <id> --agent <name> [--ttl <minutes>]`:
  пишет `.forgeplan/claims/<id>.yaml` с agent_id + TTL + timestamp,
  refuses if claim is already live and not expired
- `forgeplan_release <id> --agent <name>`:
  удаляет claim (или auto-expires по TTL)
- `forgeplan_claims --active`:
  список живых claims, сортировка по expiry

**Core changes**:
- `Claim` struct в forgeplan-core + `.forgeplan/claims/` dir (gitignored like state/)
- `domain` поле в frontmatter (расширение existing `tags`): canonical values
  frontend / backend / api / infra / docs / testing / general
- `affected_files` в frontmatter: уже у нас частично есть, формализовать обязательность для P0-P1 PRD
- `dispatch_plan` в forgeplan-core::routing — сборщик из graph+order+blocked+overlap

**Coordination primitives**:
- File lock на LanceDB writes (`.forgeplan/.lock` via `fs2::FileExt`):
  сериализует `store.create_artifact` + `update_artifact` + `next_id`.
  Concern только под lock — reads остаются concurrent.
- Agent identity в activity log: MCP `clientInfo` уже приходит, сейчас
  пишется только `client_info`. Добавить в frontmatter `last_modified_by`.

**Integration**:
- `forgeplan_health` surface active claims + expired unreleased claims
- `_next_action` в `forgeplan_get` показывает кто claimed (если есть)

### Out of Scope

- **Agent-to-agent direct communication** — оркестратор единственный coordinator
- **Cross-machine / cross-clone coordination** — только один shared workspace
- **Skill-based auto-matching с ML** — простой csv-string match, не ML
- **Rebalancing когда агент done** — один dispatch call, потом snapshot плана
- **Priority queues** — обычный topological sort + first-ready-wins
- **Distributed consensus** — git IS consensus layer (через PR/merge)
- **Enforcement** — tools НЕ блокируют работу без claim (advisory); orchestrator может override
- **Real-time filesystem notifications** (inotify / fsevents) — polling через MCP достаточно для 2-5 agents
- **Full concurrency fix для `phase/store::advance_phase`** read-modify-write window —
  remains as known limitation (2-5 agents на разных артефактах не триггерит)
- **CLI parity** — только MCP tools в первом инкременте

### Growth Vision

- **v0.25 — enforcement mode**: `forgeplan_dispatch --strict` отказывает отдавать артефакт
  без наличия skill match у агента; `forgeplan_update` требует active claim для modification
- **v0.26 — cross-epic dispatch**: план на уровне workspace, не только Epic
- **v0.27 — capability learning**: автоматический skill profiling агентов из
  activity log (какие domains этот agent_id успешно закрывал раньше)
- **Orchestra integration**: dispatch call создаёт Task'и в Orchestra автоматически

---

## User Journeys

### Journey 1: Orchestrator getting a 3-agent plan for Epic work

**Цель**: оркестратор хочет разделить EPIC-005 child PRDs между 3 sub-agents.

| Шаг | Действие оркестратора | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan_dispatch --agents 3 --epic EPIC-005` | Plan JSON с 3 buckets + serial queue + reasoning | 1 round-trip |
| 2 | Читает план: agent1 → PRD-061 (brownfield), agent2 → PRD-062 (hotfix), agent3 → PRD-063 (research) | — | parallel-safe по dep graph и affected_files |
| 3 | Каждому sub-agent'у передаёт: "работай над <ID>" | Sub-agent вызывает `forgeplan_claim <ID> --agent <self>` | soft signal |
| 4 | Sub-agent'ы работают параллельно в своих worktrees / с file lock на LanceDB | — | no conflict |
| 5 | Agent done → вызывает `forgeplan_release <id>` | Claim removed | slot свободен |
| 6 | Orchestrator re-dispatch при новых готовых tasks | — | optional |

### Journey 2: Sub-agent reads claim status before taking work

**Цель**: sub-agent хочет взять `PRD-X` но вдруг кто-то уже работает.

| Шаг | Действие | Ответ | Заметки |
|-----|---------|-------|---------|
| 1 | `forgeplan_claims --active` | List of live claims with agent_id + expires_at | — |
| 2 | Видит что PRD-X claimed agent-orch, agent-worker-1 expires in 25m | — | informed decision |
| 3 | Берёт другой unclaimed PRD-Y | `forgeplan_claim PRD-Y --agent self --ttl 30m` | lease set |
| 4 | После работы `forgeplan_release PRD-Y` | Claim removed | — |

### Journey 3: File overlap detection prevents conflict

**Цель**: предотвратить что два агента получают PRD'ы которые оба меняют одни файлы.

| Шаг | Действие orchestrator'а | Ответ | Заметки |
|-----|------------------------|-------|---------|
| 1 | `forgeplan_dispatch --agents 2` | Plan: agent1→PRD-056 (affected: crates/forgeplan-core), agent2→PRD-052 (affected: apps/website) | disjoint files |
| 2 | Plan показывает "PRD-054 и PRD-055 отложены в serial: оба меняют crates/forgeplan-mcp" | — | dispatcher предотвратил ложный параллелизм |
| 3 | Agent1 + agent2 работают параллельно | — | safe |
| 4 | Когда один done, re-dispatch раздаёт следующий из serial queue | — | — |

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | System can generate dispatch plan for N agents via `forgeplan_dispatch --agents N` returning buckets + serial queue + reasoning | Journey 1 |
| FR-002 | Core | Must | Dispatch plan excludes artifacts with affected_files overlap > threshold (default 0.3 Jaccard) | Journey 3 |
| FR-003 | Core | Must | Dispatch plan respects artifact dependency graph (blocked artifacts in serial queue, not parallel) | Journey 1 |
| FR-004 | Core | Must | Agent can claim artifact via `forgeplan_claim <id> --agent <name> [--ttl <mins>]`; refuses if already claimed non-expired | Journey 2 |
| FR-005 | Core | Must | Agent can release claim via `forgeplan_release <id>`; claims auto-expire by TTL (default 30m) | Journey 2 |
| FR-006 | Core | Must | Agent can list active claims via `forgeplan_claims --active` with agent_id + expires_at | Journey 2 |
| FR-007 | Core | Must | LanceDB writes serialized via workspace-level file lock (`.forgeplan/.lock`) to prevent concurrent corruption | — |
| FR-008 | Core | Must | `next_id` allocation inside file lock — two concurrent `forgeplan_new` calls get different IDs | — |
| FR-009 | Core | Must | MCP `client_info` (`name` + `version`) captured in `last_modified_by` field of artifact frontmatter on update | — |
| FR-010 | UX | Should | Artifacts support `domain` field in frontmatter (enum: frontend, backend, api, infra, docs, testing, general) | Journey 1 |
| FR-011 | UX | Should | `forgeplan_dispatch --skills csv` matches domain to agent skills (disjoint domains → parallel-safe) | Journey 1 |
| FR-012 | UX | Should | `forgeplan_health` includes `active_claims` + `expired_unreleased_claims` advisory sections | — |
| FR-013 | UX | Should | `forgeplan_get` `_next_action` includes claim info (claimed by, expires_at) when present | Journey 2 |
| FR-014 | Safety | Must | Claims are gitignored (like state/ logs/ trash/) — per-workspace runtime state | — |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | dispatch computation shall complete within budget | p95 < 500ms | 100 artifacts, 10 edges avg | criterion bench |
| NFR-002 | Backward Compatibility | No breaking changes in existing tool schemas | 0 | `cargo test` + pre-v0.23.0 integration | full workspace test run |
| NFR-003 | Concurrency | LanceDB writes shall be serialized per workspace | no corruption | 10 concurrent `forgeplan_new` calls | multi-process stress test |
| NFR-004 | Safety | Claim TTL shall auto-expire crashed agent holds | < 30m max hold | agent killed mid-work | simulate kill + check |
| NFR-005 | Observability | dispatch_plan response includes reasoning — why each artifact was grouped / deferred | human-readable | every response | code review |

---

## Acceptance Criteria

### AC-1: Dispatch plan produces parallel bucket for disjoint artifacts

```gherkin
Given workspace has PRD-A (affected: crates/cli), PRD-B (affected: apps/website), PRD-C (affected: crates/cli + crates/core), all status=draft, no blockers
When orchestrator calls `forgeplan_dispatch --agents 2`
Then response contains buckets[0]={PRD-A} and buckets[1]={PRD-B} (disjoint file sets)
And serial_queue contains PRD-C (file-overlap with PRD-A → deferred)
And reasoning field explains why PRD-C was deferred
```

### AC-2: Claim prevents double-work

```gherkin
Given PRD-X has no active claim
When agent-1 calls `forgeplan_claim PRD-X --agent agent-1 --ttl 30`
Then claim is written to .forgeplan/claims/PRD-X.yaml
And `forgeplan_claims --active` shows PRD-X claimed by agent-1

Given agent-1 claim is still live
When agent-2 calls `forgeplan_claim PRD-X --agent agent-2`
Then response is error with message including agent-1 and expires_at
And no second claim file is written
```

### AC-3: TTL expiration releases stale claim

```gherkin
Given PRD-Y claimed by agent-1 with ttl=1 minute, written 2 minutes ago
When agent-2 calls `forgeplan_claim PRD-Y --agent agent-2`
Then expired claim is ignored
And new claim is written for agent-2
And `forgeplan_claims --active` shows only agent-2 claim
```

### AC-4: Concurrent `forgeplan_new` produces unique IDs

```gherkin
Given workspace has 5 PRDs (last is PRD-005)
When 3 agents simultaneously call `forgeplan_new prd "..."` (via MCP)
Then agent-1 receives PRD-006, agent-2 receives PRD-007, agent-3 receives PRD-008
And no two receive the same ID
And all three markdown files exist on disk
```

### AC-5: Agent identity in frontmatter on update

```gherkin
Given PRD-Z exists without last_modified_by field
When agent "orchestrator/1.0" calls `forgeplan_update PRD-Z --body "..."`
Then PRD-Z markdown frontmatter contains `last_modified_by: orchestrator/1.0`
And `last_modified_at` is set to current RFC3339 timestamp
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| PRD-056 Phase state machine | Internal | Active (v0.23.0 shipped) | — |
| EPIC-005 umbrella | Internal | Draft | — |
| fs2 crate for file lock | External | Ready (workspace dep TBD) | — |
| Existing `forgeplan_graph`/`order`/`blocked` | Internal | Ready | — |
| MCP clientInfo protocol field | External | rmcp 1.3.0 supports it | — |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | File lock становится bottleneck при >10 agents | Low | Medium | Target scale 2-5; LanceDB reads stay concurrent; lock только write-path | impl |
| R-2 | `affected_files` не заполнен надёжно → overlap detection fails open | High | Medium | Default "treat as overlap if either has no files listed" — safe bias toward serial | impl + docs |
| R-3 | TTL слишком короткий → legit long-running agent теряет claim | Medium | Low | Allow `--ttl` до 24h; default 30m balance safety + long work | ux |
| R-4 | TTL слишком длинный → crashed agent блокирует работу часами | Medium | Medium | Orchestrator может force-release via `forgeplan_release <id> --force` с reason logged | impl |
| R-5 | Agent identity collision (два sub-agents оба "worker-1") | Low | Low | Require unique agent_id per active claim; dispatcher refuses identical names в плане | impl |
| R-6 | Dispatch даёт stale plan между call и claim (новый PRD создан meanwhile) | Medium | Low | Plan содержит `generated_at`; orchestrator re-dispatches если claim fails | ux |
| R-7 | Multi-agent audit miss (этот PRD код не audited 2-3 агентами) | Medium | High | MANDATORY: 3-4 agent audit panel перед PR merge (как на PRD-056) | process |

---

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-005 Phase state machine umbrella | based_on (parent Epic) | Draft |
| PRD-056 Phase state machine (advisory) | informs (uses same workspace/state pattern) | Active (v0.23.0) |
| Future PRD Full Phase Enforcement | informs (dispatcher will later use phase state for eligibility check) | Planned |

## Affected Files

- `crates/forgeplan-core/src/dispatch/mod.rs` (new module)
- `crates/forgeplan-core/src/dispatch/plan.rs` (compute_dispatch_plan fn)
- `crates/forgeplan-core/src/claim/mod.rs` (new module — claim/release/list)
- `crates/forgeplan-core/src/artifact/frontmatter.rs` (add `domain`, `last_modified_by` fields)
- `crates/forgeplan-core/src/db/store.rs` (wrap writes в workspace lock)
- `crates/forgeplan-core/src/workspace.rs` (expose `workspace_lock` helper)
- `crates/forgeplan-mcp/src/server.rs` (new tools: dispatch, claim, release, claims)
- `crates/forgeplan-mcp/src/server.rs` (capture client_info в update hooks)
- `.gitignore` (add `.forgeplan/claims/`)
- `CHANGELOG.md`

---

> **Next step**: validate PRD-057 → ADI (3 hypotheses) → Code (increment 1: file lock + agent identity).

