---
depth: standard
id: PRD-078
kind: prd
links:
- target: PROB-072
  relation: informs
- target: PROB-067
  relation: informs
- target: PROB-073
  relation: informs
- target: ADR-003
  relation: informs
status: active
title: MCP worktree-aware projection routing
---

# PRD-078: MCP worktree-aware projection routing

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/7  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/7  (  0%)
```

---

## Executive Summary

### Vision

MCP server маршрутизирует projection writes в worktree, из которого пришёл вызов, а не в server's startup CWD — устраняя single point of failure для multi-worktree AI agent pipelines.

### Problem

По PROB-072: MCP server forgeplan фиксирует CWD на старте (`std::env::current_dir()` в `main.rs:11`). Когда subagent работает в worktree W и вызывает `forgeplan_new`, projection пишется в **main repo's** `.forgeplan/<kind>s/`, не в W's. Guardian приходит после, ищет файл в worktree, не находит — отправляет архитектора переделывать. Получается loop, который физически не закрывается без core fix.

**Impact**:
- Один из живых пользователей запускает **19 git worktrees** параллельно во время спринта — это не edge case, а его обычный рабочий режим
- Без core fix multi-worktree pipelines требуют plugin-layer workaround (forgeplan-marketplace commit `a9a825c`) — дублирование responsibility, не масштабируется на разные agent frameworks
- Adoption блокер для team workflows с несколькими параллельными фичами

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| AI subagent в worktree | Запускается из feature worktree, делает MCP-вызовы для создания/линковки артефактов | Артефакты создаются «не в той папке», Guardian отправляет переделывать |
| Pipeline orchestrator (lead agent) | Спавнит N subagents в M worktrees, координирует через forgeplan | Не может доверять projection без post-hoc verify-in-both-locations check |
| Single-worktree CLI user | Работает в обычном репо без worktrees | Не должен ничего знать про worktree-detection логику; backward compat обязателен |

### Differentiators

- **Core-level fix** vs plugin-layer workaround — решение работает с любым agent framework (Claude Code, OpenCode, Cursor, custom), не требует cooperation от каждого
- **Smart detection** — error эмитится только в multi-worktree env; single-worktree user не платит за фичу, которая ему не нужна
- **Failure visibility** — нельзя silent fallback'нуть; error выходит через MCP response path (proven channel)

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Silent fallback в multi-worktree env устранён | Количество tool calls которые пишут в main repo при subagent работе в worktree | N (точное число неизвестно, но user catches via dual-location verify) | 0 | v0.33 release | Integration test: spawn MCP server в /repo, вызвать `forgeplan_new` из /repo-wt-feature, проверить что projection в /repo-wt-feature/.forgeplan/ |
| SC-2 | Detection overhead в single-worktree env приемлем | p95 latency overhead per tool call от multi-worktree detection | 0ms (нет detection) | <5ms | До PRD activate | criterion bench `crates/forgeplan-core/benches/workspace_detection.rs` (cross-link с PROB-073) |
| SC-3 | Backward compat для single-worktree flows | Test suite regression count | 3084 tests pass | 3084+ tests pass, 0 failures | Каждый PR в этой ветке | `cargo test --workspace` |
| SC-4 | User's plugin-layer workaround можно депрекейтить | Дополнительные verify-in-both-locations checks нужны | Required (dual-location verify hook) | Not required | После активации PRD | Verify на user's feature branch — `verify-projection.sh` упрощается до single-location check |

---

## Product Scope

### MVP (In-Scope)

- **H1**: Optional `workspace: Option<String>` параметр на mutating MCP tools — `NewParams`, `LinkParams`, `UpdateParams`
- **H2**: `FORGEPLAN_WORKSPACE` env var, читаемый **lazy at tool-call time** (не at server startup)
- **Resolution chain** для каждого tool call: `workspace param` → `FORGEPLAN_WORKSPACE env` → `std::env::current_dir()` (current behaviour)
- **Multi-worktree detection** через `git rev-parse --git-common-dir` ≠ `--show-toplevel` ИЛИ `git worktree list | wc -l > 1`
- **Option E (FPF verdict)**: Error response `MCP -32602` если detection срабатывает AND workspace не передан. Error message содержит actionable suggestion с auto-detected правильным путём
- **Per-workspace lock** — `acquire_workspace_lock` keyed на resolved path, не на static `workspace_root` (mitigates PROB-067 race)
- **Resolved workspace path** возвращается в каждом success response (для agent visibility / debug)

### Out of Scope

- **H3** (MCP `initialize` workspaceFolders / git autodetect без explicit signal) — отвергнут ADI как Low confidence: per-session limit, brittle, зависит от MCP client implementation
- **Plugin-layer enforcement** в agent definitions — handled separately by clients; этот PRD про core
- **Windows path edge cases** в detection logic — defer до Windows commitment (SEC-003 в PROB-070)
- **Config-toggle `workspace_mismatch_policy: error | warn`** (D' fallback) — defer to Growth Vision, не нужен в MVP
- **CLI flag `--workspace`** для `forgeplan` binary — CLI уже работает с cwd корректно; scope только MCP

### Growth Vision

- **Config toggle** для soft-mode (D') если cross-worktree convenience use case всплывёт после MVP
- **Session-level detection cache** — если bench покажет >5ms overhead, добавить cache per MCP session (re-check on session start, not per call)
- **`workspace` param на read-only tools** — пока scope только mutating, но read tools тоже могут benefit'нуть от per-call routing

---

## User Journeys

### Journey 1: AI subagent в worktree создаёт артефакт

**Цель пользователя**: subagent в `~/repo-wt-feature-x` должен создать PRD через MCP, артефакт должен appear в `~/repo-wt-feature-x/.forgeplan/prds/`.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | Subagent в `~/repo-wt-feature-x` вызывает `forgeplan_new` с `workspace="~/repo-wt-feature-x"` | Server разрешает workspace через chain step 1 (param), пишет projection в `~/repo-wt-feature-x/.forgeplan/prds/` | Happy path |
| 2 | Guardian (другой subagent) в `~/repo-wt-feature-x` вызывает `forgeplan_get` с `workspace="~/repo-wt-feature-x"` | Server резолвит, читает из правильного worktree, возвращает артефакт | Loop не образуется |
| 3 | Subagent забыл передать `workspace` | Server detect'ит multi-worktree, возвращает MCP error -32602 с `workspace: ~/repo-wt-feature-x` suggestion | Failure visibility (FPF Option E) |

**Результат**: артефакты лендятся в правильном worktree; Guardian не отправляет переделывать; silent fallback структурно невозможен.

### Journey 2: Single-worktree CLI user (backward compat)

**Цель пользователя**: обычный пользователь в `~/repo` создаёт PRD через `forgeplan new prd "..."` или через MCP, без знания про worktrees.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | User в `~/repo` вызывает `forgeplan new prd "Title"` (CLI) или MCP `forgeplan_new` без `workspace` параметра | Server detection: `git rev-parse --git-common-dir` == `--show-toplevel` → single-worktree → пропускает error gate → fallback на cwd (chain step 3) | Поведение неизменно |
| 2 | Artifact appear в `~/repo/.forgeplan/prds/` | Совпадает с поведением v0.32.x | 0 регрессий |

**Результат**: zero behavioural change для existing users; никаких новых концепций learn.

### Journey 3: CI runner с строгим режимом (опционально)

**Цель пользователя**: CI runner хочет fail loudly если кто-то забыл передать `workspace` даже в single-worktree контексте (defensive coding).

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | CI runner устанавливает `FORGEPLAN_WORKSPACE=/workspace/repo` перед запуском MCP server | Server использует env var через chain step 2 | Opt-in strict |
| 2 | MCP tool calls без `workspace` param идут через env var | Все calls попадают в `/workspace/repo` | Predictable |

**Результат**: CI environments могут pin workspace без зависимости от cwd-detection.

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | MCP server resolves workspace per tool call через chain: `params.workspace` → `FORGEPLAN_WORKSPACE` env → `std::env::current_dir()` | Journey 1, 2, 3 |
| FR-002 | Core | Must | `NewParams`, `LinkParams`, `UpdateParams` accept optional `workspace: Option<String>` field | Journey 1 |
| FR-003 | Core | Must | Server reads `FORGEPLAN_WORKSPACE` env var **at tool-call time**, не at startup | Journey 3 |
| FR-004 | Core | Must | Server detects multi-worktree environment через `git rev-parse --git-common-dir` ≠ `--show-toplevel` | Journey 1, 2 |
| FR-005 | UX | Must | Server returns MCP error `-32602` when multi-worktree detected AND `workspace` not provided AND env var not set; error message содержит auto-detected suggested path | Journey 1 |
| FR-006 | Core | Must | `acquire_workspace_lock` использует resolved path, не static `workspace_root` (per-workspace isolation) | Journey 1 (concurrency) |
| FR-007 | UX | Should | Resolved workspace path возвращается в `resolved_workspace` поле каждого success response | Journey 1, 2 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | Multi-worktree detection overhead per tool call | < 5ms p95 | Cold + warm | criterion bench `workspace_detection_bench` |
| NFR-002 | Compatibility | Existing single-worktree workflows | 0 behavioural changes | Регрессия test suite | `cargo test --workspace` count: 3084 → ≥3084 |
| NFR-003 | Operability | Error message содержит actionable suggestion | Включает auto-detected правильный path | Multi-worktree env + missing workspace | Integration test parses error body |
| NFR-004 | Operability | Resolution chain step выбранный server'ом доступен в response | `resolved_via: "param" \| "env" \| "cwd"` field в response | Каждый tool call | Integration test inspects response |

---

## Acceptance Criteria

### AC-1: Subagent в worktree создаёт артефакт

```gherkin
Given существует worktree /tmp/repo-wt-feature-x с инициализированным .forgeplan/
And MCP server запущен с CWD = /tmp/repo (main, не worktree)
When subagent вызывает forgeplan_new(kind="prd", title="Test", workspace="/tmp/repo-wt-feature-x")
Then artifact файл создаётся в /tmp/repo-wt-feature-x/.forgeplan/prds/PRD-NNN-*.md
And response содержит resolved_workspace = "/tmp/repo-wt-feature-x"
And response содержит resolved_via = "param"
And никакой файл не создаётся в /tmp/repo/.forgeplan/
```

### AC-2: Backward compatibility — single-worktree user

```gherkin
Given существует обычный repo /tmp/repo без worktrees
And MCP server запущен с CWD = /tmp/repo
When subagent вызывает forgeplan_new(kind="prd", title="Test") БЕЗ workspace param и БЕЗ env var
Then artifact файл создаётся в /tmp/repo/.forgeplan/prds/PRD-NNN-*.md (как в v0.32.x)
And response содержит resolved_via = "cwd"
And никакого warning или error не возвращается
```

### AC-3: Multi-worktree без workspace — explicit error

```gherkin
Given worktree /tmp/repo-wt-feature-x linked к main /tmp/repo
And MCP server запущен с CWD = /tmp/repo-wt-feature-x
And env var FORGEPLAN_WORKSPACE не установлен
When subagent вызывает forgeplan_new(kind="prd", title="Test") БЕЗ workspace param
Then server возвращает MCP error -32602
And error message содержит "multi-worktree detected"
And error message содержит actionable suggestion "workspace: /tmp/repo-wt-feature-x"
And никакого файла не создаётся
```

### AC-4: CI strict mode через env var

```gherkin
Given MCP server запущен с env var FORGEPLAN_WORKSPACE = /opt/ci-workspace
And /opt/ci-workspace содержит инициализированный .forgeplan/
When любой tool call приходит без workspace param
Then server использует /opt/ci-workspace
And response содержит resolved_via = "env"
```

### AC-5: User's workaround можно депрекейтить

```gherkin
Given user's plugin verify-projection.sh имеет dual-location check
When PRD-078 имплементация активирована
Then verify-projection.sh может проверять только single location (resolved workspace из response)
And никаких регрессий в user's pipeline (verified empirically)
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| PROB-067 (workspace lock race в parallel worktrees) | Internal — informs design FR-006 | Active | Same team |
| PROB-073 (MCP per-call latency profile) | Internal — shares criterion bench infra; NFR-001 zit ties latency budget | Active, paired sprint | Same team |
| `git` binary в PATH | External (runtime) | Standard assumption | — |
| ADR-003 (file-first invariant) | Internal — must be preserved | Active | Same team |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | Latency cost git rev-parse на каждом MCP call превышает NFR-001 (5ms p95) | Medium | High | (a) криterion bench перед PRD activate; (b) если >5ms — добавить session-level cache; (c) ties с PROB-073 — общая infra | Team |
| R-2 | Cross-worktree convenience scenarios — agent в worktree-A хочет осознанно писать в main (e.g., shared artifact). Error-on-detect ломает этот use case | Low | Medium | Agent passes `workspace=<main path>` явно. Если случаев больше чем ожидаем — добавить config toggle (Growth Vision D') | Team |
| R-3 | Error message wording не actionable — agent не понимает что делать | Low | High | NFR-003 + AC-3 enforce wording с auto-detected suggested path. Integration test parses error body | Team |
| R-4 | Plugin-layer workaround (forgeplan-marketplace a9a825c) конфликтует с core fix (double-handling) | Medium | Low | User уведомляется при v0.33 release; SC-4 acceptance включает empirical verify на его branch | Team + user |
| R-5 | MCP client (не Claude Code) не пропускает env var до child process — strict mode ломается в OpenCode/Cursor | Low | Medium | Evidence gap: short test на OpenCode/Cursor до activate. Documented in `docs/operations/MCP-CLIENTS.ru.md` | Team |

---

## Timeline

| Milestone | Target Date | Description |
|-----------|-------------|-------------|
| PRD draft validated | 2026-05-23 | MUST sections заполнены, `forgeplan validate` PASS |
| Latency bench evidence | 2026-05-24 | NFR-001 measured, R-1 closure |
| RFC architecture | 2026-05-25 | Implementation phases, file ownership grid для team |
| ADR (E over D' over A) | 2026-05-25 | Decision record для будущего audit trail |
| Code complete | 2026-05-27 | FR-001..007 implemented, `cargo test` PASS |
| Audit complete | 2026-05-28 | 2+ adversarial agent reviews, findings closed |
| Evidence + activate | 2026-05-29 | EvidencePack linked, PRD-078 → active |
| Merged в dev | 2026-05-29 | PR merged with user approval |

---

## Stakeholders

| Role | Name | Sign-off |
|------|------|----------|
| Product Owner | explosivebit | [ ] |
| Engineering Lead | explosivebit | [ ] |
| User (dogfood feedback) | TBD (известный по PROB-072 signal) | [ ] |

---

## Affected Files

- `crates/forgeplan-mcp/src/main.rs` — server startup, env var handling
- `crates/forgeplan-mcp/src/server.rs` — tool dispatcher, resolution chain, detection
- `crates/forgeplan-mcp/src/convert.rs` — `NewParams`/`LinkParams`/`UpdateParams` schema additions
- `crates/forgeplan-core/src/workspace/init.rs` — `find_workspace` extension if needed
- `crates/forgeplan-core/src/workspace/lock.rs` — `acquire_workspace_lock` keyed on resolved path
- `crates/forgeplan-core/benches/workspace_detection.rs` — NEW criterion bench (shared с PROB-073)
- Integration tests: `crates/forgeplan-mcp/tests/worktree_routing_e2e.rs` — NEW

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| PROB-072 | informs (parent problem signal) | draft |
| PROB-067 | informs (workspace lock race — same surface area) | active |
| PROB-073 | paired (latency budget; shared bench infra) | draft |
| ADR-003 | preserves (file-first invariant must hold) | active |
| RFC-NNN | will define (implementation phases, worker file ownership) | TBD |
| ADR-NNN | will record (E over D' over A — FPF verdict) | TBD |

---

## Reasoning Trace (для audit trail)

Этот PRD — результат двух reasoning cycles:

1. **ADI cycle on PROB-072** (gemini-3.1-pro-preview, 2026-05-22):
   - Abduction: 3 гипотезы (H1=per-call param, H2=env var, H3=git autodetect)
   - Deduction: H1 backward-compatible + per-call routing; H2 fails if MCP shared across subagents; H3 brittle (per-session limit)
   - Induction: H1 supported empirically; H2 partially; H3 refuted by MCP spec
   - Verdict: H1 primary + H2 fallback. Resolution chain explicit. Confidence: High.

2. **FPF Evaluate on silent-fallback risk** (2026-05-22, после ADI):
   - Triggered by ADI risk: «если agent забыл workspace — silent fallback на main repo, та же PROB-072 опять»
   - Options: B (stderr warning), C (plugin layer), D (stderr+strict), D' (response payload warning + strict), E (error on detect), F (noop)
   - Empirical test (Claude Code MCP logs 16 days × 442 lines): stderr from forgeplan-mcp **НЕ surface'ится** → B/D rejected as proven broken
   - F-G-R scoring: E (3/3/2, Trust 0.85) > D' (3/2/2, Trust 0.80) > все остальные
   - Killer argument: PROB-072 само родилось из silent fallback; soft signal через 6 месяцев деградирует до игнора; E structurally невозможно обойти silent
   - Verdict: E primary, D' as fallback design если cross-worktree convenience use case критичен

> **Next step**: создать RFC с phased implementation + ADR documenting E choice; затем code; затем evidence; затем activate.












