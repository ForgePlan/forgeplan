---
depth: standard
id: PRD-056
kind: prd
links:
- target: EPIC-005
  relation: based_on
status: draft
title: Phase state machine (advisory) — per-artifact current_phase visibility
---

---
id: PRD-056
title: "Phase state machine (advisory) — per-artifact current_phase visibility"
status: Draft
author: gogocat
created: 2026-04-18
updated: 2026-04-18
priority: P0
depth: standard
domain: general
projectType: cli_tool
epic: EPIC-005
stepsCompleted: []
---

# PRD-056: Phase state machine (advisory) — per-artifact current_phase visibility

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/12  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/12  (  0%)
```

---

## Executive Summary

### Vision

Каждый artifact в **greenfield workflow** имеет видимую текущую фазу
методологического цикла (Shape → Validate → ADI → Code → Evidence →
Activate) — advisory метка, которую агент и человек видят в каждом
tool-ответе и health-отчёте, формируя foundation для будущего
enforcement и workflow-aware расширений.

**Scope boundary**: это **Mini-X** — первый child в Epic umbrella по
phase tracking. Покрывает только greenfield (новые артефакты с нуля).
Brownfield, audit-hotfix, research, review-fix workflows имеют свои
фазы и покрываются отдельными PRD'ами под тем же Epic.

### Problem

Сейчас Forgeplan имеет lifecycle для status (`draft → active → superseded`),
но **нет** записи о том на какой фазе методологического цикла находится
artifact. Агент может создать PRD и сразу писать код без Shape/Validate/ADI,
или активировать без Evidence (R_eff gate спасает частично, но только на
активации). Prompt в CLAUDE.md `Route → Shape → Validate → Code → Evidence
→ Activate` — это дисциплина, не enforcement.

**Impact**:
- За сессию 2026-04-18 (v0.20.0–v0.22.1) 11 PR, но 3 раза наблюдалось
  "Code без полного Shape" (PRD-055 инкременты шли с минимальной Shape-фазой,
  что пришлось компенсировать post-ship audit'ом — 2 CRITICAL + 5 HIGH).
- `_next_action` hints напоминают, но если агент не следует — ничего не
  ловит.
- R_eff gate на активации ловит только отсутствие evidence, не skip'ы
  между другими фазами.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Claude Code agent (primary) | LLM-агент работающий через MCP | Нет механизма "где я сейчас нахожусь в цикле" для self-check |
| Human developer (secondary) | Пользователь Forgeplan | Сложно понять что artifact не готов к активации (формально status=draft, но что из методологии сделано?) |
| Project auditor (tertiary) | Ревьюер / архитектор | Сейчас надо открывать PRD + искать вручную что Validate/ADI/Evidence сделано |

### Differentiators

- Advisory phase marker — **не блокирует** ни один tool, foundation для
  полного enforcement позже (v0.24.0 / PRD-0XX Full Phase Enforcement).
- Per-artifact state (не глобальный SessionState) — каждый PRD имеет
  свою фазу, multi-agent ready без блокировок.
- Auto-advancement на известных переходах — `forgeplan_new` → phase=shape,
  `forgeplan_activate` → phase=activated, `forgeplan_validate PASS` →
  advance shape → validate.
- Видим в `_next_action`, `health`, `get` responses.

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Каждый новый artifact получает `current_phase` на создании | % artifacts с state.yaml / total | 0% | 100% | On v0.23.0 ship | `ls .forgeplan/state/ \| wc -l` ≥ count(artifacts) |
| SC-2 | Health surface показывает phase-status mismatch | False-negatives в health | — | 0 | Release check | forgeplan health выводит `mismatched_phases` список |
| SC-3 | `_next_action` в `forgeplan_get` включает current phase | Tools с phase in hint / total get-tools | 0/1 | 1/1 | On ship | Manual test |
| SC-4 | Нет breaking changes в существующих tool schemas | breaking changes count | — | 0 | CI | cargo test + existing E2E suite pass |

---

## Product Scope

### MVP (In-Scope)

- Файлы state: `.forgeplan/state/<ID>.yaml` с полями `current_phase`,
  `advanced_from`, `advanced_at`, `reason`. Gitignored.
- Enum `Phase`: `shape, validate, adi, code, test, audit, evidence,
  activate, done`.
- Auto-advancement на известных tool calls:
  - `forgeplan_new <kind> <title>` → phase=shape
  - `forgeplan_validate <id>` с результатом PASS → shape → validate
  - `forgeplan_activate <id>` → phase=activated (= done)
  - `forgeplan_supersede`/`forgeplan_deprecate` → phase=done (terminal)
- Новые MCP tools:
  - `forgeplan_phase <id>` — read current phase + history
  - `forgeplan_phase_advance <id> --to <phase> [--reason]` — advisory
    set, без validation что prev phase complete
- `_next_action` enrichment: в `forgeplan_get`/`forgeplan_list` включить
  phase если есть
- Health gate extension: `forgeplan_health` выводит phase/status
  mismatches как advisory warnings (не failures): e.g. status=active
  но phase=shape → warning "скорее всего пропущен phase tracking"

### Out of Scope

- **Enforcement** — tools НЕ refuse работать не в своей фазе. Это
  следующий PRD (Full Phase Enforcement, v0.24.0 target).
- **Non-greenfield workflows**:
  - Brownfield modification (изменение существующих артефактов)
  - Audit-hotfix workflow (audit → triage → fix → regression → release)
  - Research / spike workflow (scope → explore → synthesize → report)
  - Review-fix workflow (identify_comments → address → re-push)
  - Refactor workflow (no new feature)
  - Docs-only / config-only workflows
  
  Каждый из них имеет свои phases и покрывается отдельным PRD под
  тем же Epic umbrella.
- CLI parity — только MCP tools в первом incrementе. CLI flags —
  follow-up.
- Persistence phase advancement в activity log — только state.yaml.
- Migration для существующих ~100 artifacts — будет backfill команда
  `forgeplan_phase_backfill` которая догадывается по status.
- Graph-level operations (advance all children of epic в one shot).

### Growth Vision

- **v0.24.0 Full Enforcement** (PRD-TBD): tools refuse работать не в
  своей фазе, `forgeplan_advance` с validation prev phase complete.
- **v0.25.0 Phase telemetry** (PRD-TBD): activity_log correlates
  phase transitions with agent sessions.
- **v0.26.0 Phase templates** (PRD-TBD): разные cycles для разных
  artifact kinds (PRD vs ADR vs Note имеют разные фазы).

---

## User Journeys

### Journey 1: Agent creates PRD and sees phase hints through cycle

**Цель пользователя**: Claude Code agent проходит полный цикл нового
PRD, получая на каждом шаге подсказку "ты на фазе X, следующий шаг Y".

| Шаг | Действие агента | Ответ системы | Заметки |
|-----|----------------|---------------|---------|
| 1 | `forgeplan_new prd "New feature"` | `{id: PRD-XXX, current_phase: shape, _next_action: "Phase: shape. Fill MUST sections, then forgeplan_validate."}` | auto-set shape on creation |
| 2 | Заполняет MUST sections через edit | — | No tool call — phase stays shape |
| 3 | `forgeplan_validate PRD-XXX` | `{pass: true, current_phase: validate, _next_action: "Phase: validate. Standard requires ADI. forgeplan_reason PRD-XXX."}` | auto-advance on PASS |
| 4 | `forgeplan_reason PRD-XXX` | existing ADI output + `current_phase: adi` | auto-advance |
| 5 | Code + tests + forgeplan_new evidence + forgeplan_link | existing tools + phase advance на evidence создании | graceful phase progression |
| 6 | `forgeplan_activate PRD-XXX` | `{status: active, current_phase: activated (done), _next_action: "Phase: done. Terminal — no further action."}` | terminal |

**Результат**: агент никогда не теряется "где я в цикле", видит
advisory-указание что сделано и что дальше.

### Journey 2: Developer audits workspace for skipped phases

**Цель пользователя**: Человек проверяет здоровье workspace —
какие artifacts активированы, но фактически пропустили фазы.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan_health` | `{...existing..., advisory_phase_mismatches: [{id: PRD-XXX, status: active, last_known_phase: shape, skipped: [validate, adi, evidence]}]}` | Новая секция |
| 2 | `forgeplan_phase PRD-XXX` | `{current_phase: shape, advanced_at: 2026-04-10, history: [{from: none, to: shape, at: 2026-04-10}]}` | Show history |
| 3 | Решение: fix backfill или deprecate | — | Manual decision |

**Результат**: mismatches visible, но ничего не сломано.

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | System can record current phase per artifact in `.forgeplan/state/<ID>.yaml` with history | Journey 1 |
| FR-002 | Core | Must | Agent can read current phase of an artifact via `forgeplan_phase <id>` | Journey 2 |
| FR-003 | Core | Must | Agent can advance phase manually via `forgeplan_phase_advance <id> --to <phase> [--reason]` without validation | Journey 1 |
| FR-004 | Integration | Must | System auto-sets phase=shape when `forgeplan_new` creates an artifact | Journey 1 |
| FR-005 | Integration | Must | System auto-advances phase to validate when `forgeplan_validate` returns PASS | Journey 1 |
| FR-006 | Integration | Must | System auto-advances phase to done when `forgeplan_activate`/`supersede`/`deprecate` succeed | Journey 1 |
| FR-007 | UX | Should | Agent can see current phase in `_next_action` of `forgeplan_get` and `forgeplan_list` responses | Journey 1 |
| FR-008 | UX | Should | Developer can see phase-status mismatches in `forgeplan_health` as advisory warnings | Journey 2 |
| FR-009 | Ergonomic | Could | System can backfill phase for existing artifacts via `forgeplan_phase_backfill` (infers from status) | — |
| FR-010 | Integration | Could | `forgeplan_reason` auto-advances phase to adi on successful ADI completion | Journey 1 |
| FR-011 | Safety | Must | State files are workspace-local, gitignored, never committed | — |
| FR-012 | Safety | Must | Missing state file is not an error — treated as `current_phase: unknown`, doesn't block any tool | — |
| FR-013 | Config | Must | Feature-flag in `.forgeplan/config.yaml` (`phase.enabled: bool`, default true) gates all phase tracking; when false, behavior is exact pre-v0.23.0 semantics | — |
| FR-014 | Extensibility | Should | `state.yaml` includes `workflow_type: greenfield` field (enum, default "greenfield"); future PRDs under Epic umbrella add other workflow types without breaking schema | — |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | Phase read/write shall add negligible overhead | < 5ms p95 per tool call | Local filesystem | Bench `forgeplan_phase`+auto-advance wrapper |
| NFR-002 | Backward Compatibility | Existing tool schemas shall not change semantically | 0 breaking changes | All existing tests pass | `cargo test --workspace` + existing E2E |
| NFR-003 | Durability | Phase state shall survive agent crash mid-advance | fsync per write | All write_phase calls | tokio::fs::write + sync_data |
| NFR-004 | Privacy | Phase state shall not contain sensitive data | Phase enum + timestamps only | By design | Code review |
| NFR-005 | Portability | Phase state format shall be human-readable for manual inspection | YAML | On disk | File format check |

---

## Acceptance Criteria

### AC-1: Fresh PRD creation sets phase to shape

```gherkin
Given a fresh Forgeplan workspace
When agent calls forgeplan_new prd "Test"
Then artifact PRD-XXX is created
And file .forgeplan/state/PRD-XXX.yaml exists
And state.yaml contains current_phase: shape
And response _next_action mentions phase
```

### AC-2: forgeplan_validate PASS advances phase

```gherkin
Given PRD-XXX exists with current_phase=shape and all MUST sections filled
When agent calls forgeplan_validate PRD-XXX
Then validation returns PASS
And state.yaml current_phase becomes validate
And history shows {from: shape, to: validate, at: <now>}
```

### AC-3: Missing state file does not break existing tools

```gherkin
Given PRD-XXX exists but .forgeplan/state/PRD-XXX.yaml was deleted
When agent calls forgeplan_get PRD-XXX
Then response succeeds
And _next_action includes "current_phase: unknown"
And no error is raised
```

### AC-4: Health surface shows phase-status mismatch

```gherkin
Given PRD-XXX has status=active and current_phase=shape (skip detected)
When developer calls forgeplan_health
Then response contains advisory_phase_mismatches with PRD-XXX entry
And health does not fail (exit 0, advisory only)
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| forgeplan-core::artifact | Internal | Ready | — |
| tokio::fs | External | Ready | — |
| serde_yaml | External | Ready | workspace dep |
| .gitignore update for state/ | Internal | Done in this PR | — |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | Race condition when two tools advance phase concurrently (multi-agent pre-PRD-057) | Low | Medium | Use `create_new(true)` + tmp-file + rename for phase write; log last-write-wins behavior | impl |
| R-2 | Agents relying on advisory becoming confused when Full Enforcement lands (v0.24) | Medium | Low | Clear `advisory` label in tool docs; migration note in CHANGELOG when enforcement arrives | docs |
| R-3 | Backfill wrong for existing artifacts (infers phase from status incorrectly) | Medium | Low | Backfill marks `inferred: true` flag; human can correct via phase_advance | impl |
| R-4 | State file corruption loses phase history | Low | Low | Tmp-file + rename atomic write; fallback to phase=unknown on parse error | impl |

---

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-005 Phase state machine umbrella | based_on (parent Epic — umbrella для всех phase/workflow PRDs) | Draft |
| PRD-054 Activity log | informs (activity log integration path for phase transitions) | Active |
| PRD-055 Soft-delete | informs (soft-delete pattern reused for state durability) | Active |
| Future PRD Brownfield workflow | sibling under Epic umbrella | Planned |
| Future PRD Audit-hotfix workflow | sibling under Epic umbrella (formalizes what we did ad-hoc for v0.22.1) | Planned |
| Future PRD Research workflow | sibling under Epic umbrella | Planned |
| Future PRD Review-fix workflow | sibling under Epic umbrella | Planned |
| Future PRD Full Phase Enforcement | supersedes (this Mini-X becomes enforced in v0.24.0) | Planned |

## Affected Files

- `crates/forgeplan-core/src/phase/mod.rs` (new module)
- `crates/forgeplan-core/src/phase/store.rs` (read/write state.yaml)
- `crates/forgeplan-core/src/phase/transitions.rs` (auto-advance logic)
- `crates/forgeplan-mcp/src/server.rs` (hook into existing tool handlers)
- `crates/forgeplan-core/src/health/mod.rs` (advisory mismatch detection)
- `.gitignore` (add `.forgeplan/state/`)
- `CHANGELOG.md`

---

> **Next step**: validate PRD-056 → ADI (3+ hypotheses, Standard recommended) → Code.

