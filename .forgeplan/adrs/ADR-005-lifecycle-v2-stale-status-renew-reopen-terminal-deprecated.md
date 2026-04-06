---
depth: standard
id: ADR-005
kind: adr
links:
- target: PROB-019
  relation: based_on
status: active
title: Lifecycle v2 — stale status, renew, reopen, terminal deprecated
---

---
id: ADR-005
title: "Lifecycle v2 — stale status, renew, reopen, terminal deprecated"
status: Proposed
depth: deep
valid_until: 2027-04-02
created: 2026-04-02
updated: 2026-04-02
---

# ADR-005: Lifecycle v2 — stale status, renew, reopen, terminal deprecated

## Context

E2E тестирование Sprint 2 (PROB-018) обнаружило что `deprecated → active` transition разрешён.
Анализ quint-code (первоисточник) показал что deprecated и superseded = terminal statuses.
Quint-code использует `refresh_due` промежуточный статус + `reopen` (создаёт новый артефакт) + `waive` (продлить validity).

Текущая state machine Forgeplan v0.12:
```
draft → active → superseded (terminal)
               → deprecated (terminal, BUT deprecated→active allowed = bug)
```

Проблема: AI-агент может случайно activate deprecated артефакт. Нет промежуточного состояния для "устарел но ещё не мёртв". Нет пути отмены deprecate кроме прямого воскрешения.

## Decision

**Selected**: Lifecycle v2 с `stale` + `renew` + `reopen` (из quint-code, адаптированные имена)

**Why Selected**: Соответствует quint-code (проверенная модель), чистая state machine без "воскрешения", провенанс сохраняется (deprecated = terminal навсегда).

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| A. Оставить deprecated→active | Rejected | Нарушает провенанс, AI-агент может ошибиться |
| B. Блокировать deprecated→active полностью, без stale | Rejected | Нет undo для ошибок, нет промежуточного состояния для stale evidence |
| C. deprecated→active только с --force | Rejected | Полумера — force flag обходится, state machine грязная |
| **D. stale + renew + reopen** | **Chosen** | Чистая state machine, quint-code alignment, все use cases покрыты |

## New State Machine

```
         activate           deprecate
  draft ────────→ active ────────────→ deprecated (TERMINAL)
                    │
                    │ supersede
                    ├──────────────→ superseded (TERMINAL)
                    │
                    │ (auto: valid_until expired OR manual)
                    ▼
                  stale
                 ╱     ╲
         renew  ╱       ╲  reopen
               ╱         ╲
           active    deprecated + NEW draft (linked)
```

### Transitions
| From | To | Command | Condition |
|------|----|---------|-----------|
| draft | active | `activate` | validation gate |
| active | superseded | `supersede --by NEW` | terminal |
| active | deprecated | `deprecate --reason` | terminal |
| active | stale | `stale` (manual) or auto (valid_until expired) | - |
| stale | active | `renew --reason --until` | extends validity |
| stale | deprecated + NEW draft | `reopen --reason` | creates new linked artifact |

### NOT allowed (removed)
- ~~deprecated → active~~ (was un-deprecate, now terminal)
- ~~superseded → anything~~ (already terminal)

## Consequences

### Positive
- Чистая, предсказуемая state machine
- AI-агент не может случайно воскресить deprecated артефакт
- `stale` detection интегрируется с `forgeplan health` (blind spots)
- `reopen` сохраняет lineage — новый артефакт ссылается на старый

### Negative (trade-offs)
- Breaking change: существующие скрипты с `activate` на deprecated сломаются
- Новый статус `stale` — нужно обновить все фильтры (list, search, health, score)
- `reopen` создаёт новый артефакт — ID меняется (PRD-022 → PRD-023)

### Risks
- Миграция: существующие deprecated артефакты не затронуты (terminal = terminal)
- `stale` detection может быть шумным если много артефактов без valid_until

## Invariants

- deprecated и superseded = TERMINAL. Никогда не переходят в другие статусы.
- `reopen` ВСЕГДА создаёт новый артефакт. Старый → deprecated.
- `renew` требует `--reason` и `--until` (нельзя renew без обоснования).
- `stale` артефакт виден в `health` как "needs attention".

## Evidence Requirements

- Unit tests для всех transitions (positive + negative)
- E2E: `activate` на deprecated → error
- E2E: `renew` на stale → active
- E2E: `reopen` создаёт новый + deprecates old + links
- Regression: 699+ existing tests pass

## Affected Files

| File | Change |
|------|--------|
| `crates/forgeplan-core/src/lifecycle/transitions.rs` | New state machine |
| `crates/forgeplan-core/src/lifecycle/mod.rs` | `renew()`, `reopen()` |
| `crates/forgeplan-core/src/stale/mod.rs` | Auto stale detection update |
| `crates/forgeplan-cli/src/commands/renew.rs` | NEW: CLI command |
| `crates/forgeplan-cli/src/commands/reopen.rs` | NEW: CLI command |
| `crates/forgeplan-cli/src/main.rs` | Register new commands |
| `crates/forgeplan-mcp/src/server.rs` | MCP tools: renew, reopen |
| `crates/forgeplan-core/src/health/mod.rs` | Show stale in health |

## Rollback Plan

**Triggers**: Если `stale` статус создаёт confusion у пользователей или ломает MCP workflow.

**Steps**:
1. Revert transitions.rs к v1 (restore deprecated→active)
2. Remove stale status, renew, reopen commands
3. Keep self-link guard (PROB-019) — independent fix

**Blast Radius**: Lifecycle only. scoring, validation, search не затронуты.

## Implementation Plan

### Phase 1: State machine + transitions (core) ✅
- [x] **1.1** Add `stale` status to types + transitions
- [x] **1.2** Remove `deprecated → active` transition
- [x] **1.3** Add transitions: `active → stale`, `stale → active` (renew), `stale → deprecated` (reopen)
- [x] **1.4** Tests for all transitions (15 positive + negative)

### Phase 2: Core logic (renew + reopen) ✅
- [x] **2.1** `lifecycle::renew()` — stale → active, extend valid_until, date validation, reason sanitization
- [x] **2.2** `lifecycle::reopen()` — stale/active → deprecated + create new draft with lineage, atomicity guard
- [x] **2.3** Tests for renew + reopen (10 tests incl. edge cases)

### Phase 3: CLI commands ✅
- [x] **3.1** `forgeplan renew <id> --reason --until`
- [x] **3.2** `forgeplan reopen <id> --reason`
- [ ] **3.3** Update `forgeplan health` to show stale artifacts (deferred)
- [ ] **3.4** Update `forgeplan stale` command to set status (deferred)

### Phase 4: MCP + integration (deferred)
- [ ] **4.1** MCP tools: forgeplan_renew, forgeplan_reopen
- [x] **4.2** E2E smoke tests (done in Sprint 3)
- [x] **4.3** Update CLAUDE.md lifecycle docs (done in Sprint 3)

## AI Guidance

- deprecated and superseded are TERMINAL — never allow transition out
- When AI agent encounters deprecated artifact, suggest `reopen` not `activate`
- `stale` is not an error — it's a signal to review. `health` command should highlight it.
- `renew` requires justification. Don't auto-renew without evidence.

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PROB-019 | ProblemCard | based_on |



