---
id: ADR-{NNN}
title: "{title}"
status: Proposed
depth: standard / deep / critical
valid_until: YYYY-MM-DD
problem_ref: PROB-{NNN}
created: YYYY-MM-DD
updated: YYYY-MM-DD
---

# ADR-{NNN}: {Decision Title}

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Context

Какой был контекст и проблема. Ссылка на RFC/PROB если есть.

## Decision

Что именно решили. Кратко и чётко.

**Selected**: {название выбранного варианта}

**Why Selected**: {обоснование выбора}

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| A | Rejected | ... |
| B | **Chosen** | ... |
| C | Rejected | ... |

## Consequences

### Positive
- ...

### Negative (trade-offs)
- ...

### Risks
- ...

<!-- Depth: standard+ — обязательно для standard, deep, critical -->

## Invariants

{Что ДОЛЖНО выполняться всегда, независимо от реализации}

- ...
- ...

## Evidence Requirements

{Что измерить/доказать для подтверждения решения}

- ...
- ...

## Valid Until

**Дата**: `valid_until` из frontmatter

**Обоснование TTL**: {почему выбран именно такой срок}

**Refresh Triggers** (когда пере-оценить досрочно):
- ...
- ...

<!-- Depth: deep+ — обязательно для deep, critical -->

## Pre-conditions (чеклист ДО реализации)

- [ ] ...
- [ ] ...

## Post-conditions (Definition of Done)

- [ ] ...
- [ ] ...

## Admissibility

{Что НЕ допускается в рамках этого решения}

- NOT: ...
- NOT: ...

## Rollback Plan

**Triggers** (когда откатывать):
- ...
- ...

**Steps** (шаги отката):
1. ...
2. ...

**Blast Radius**: {масштаб влияния отката}

## Weakest Link

{Оценка самого слабого звена выбранного решения — R_eff = min(evidence_scores)}

## Affected Files

| File | Baseline Hash |
|------|---------------|
| | |

<!-- /Depth: deep+ -->

## AI Guidance

> Правила для AI-агентов при работе с этим решением.

- Prefer this pattern in all new code
- Do not introduce alternative approaches without a new RFC
- When generating code, assume this decision is binding
- If a task conflicts with this ADR, raise it explicitly

## Implementation Plan

### Phase 0: Foundation
- [ ] **0.1** ...
- [ ] **0.2** ...

### Phase 1: Core
- [ ] **1.1** ...
- [ ] **1.2** ...

## Implementation Log

<!-- Add wave entries as sprints are completed:

### Wave 1 — YYYY-MM-DD
| Task | Teammate | Status | Files |
|------|----------|--------|-------|
| 0.1 | ... | Done | ... |
-->

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| RFC-{NNN} | RFC | based_on |
| PROB-{NNN} | ProblemCard | based_on |
| SOL-{NNN} | SolutionPortfolio | based_on |
| SPEC-{NNN} | Spec | implements |
