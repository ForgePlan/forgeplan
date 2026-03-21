# DEC-{NNN}: {Decision Title}

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | YYYY-MM-DD |
| Mode | tactical / standard / deep |
| Valid Until | YYYY-MM-DD |
| Problem | PROB-{NNN} |
| Portfolio | SOL-{NNN} |

---

## 1. Problem Frame

{Автоматически из связанного ProblemCard}

**Signal**:
**Constraints**:
**Acceptance**:

## 2. Decision (контракт)

**Selected**: {название выбранного варианта}

**Why Selected**: {обоснование выбора}

### Invariants (что ДОЛЖНО выполняться всегда)
-
-

### Pre-conditions (чеклист ДО реализации)
- [ ]
- [ ]

### Post-conditions (Definition of Done)
- [ ]
- [ ]

### Admissibility (что НЕ допускается)
- NOT:
- NOT:

## 3. Rationale

### Comparison

| Variant | Verdict | Reason |
|---------|---------|--------|
| V1 | Selected | |
| V2 | Rejected | |
| V3 | Rejected | |

### Weakest Link

{Оценка самого слабого звена выбранного решения}

### Evidence Requirements

{Что измерить/доказать для подтверждения решения}

-
-

## 4. Consequences

### Rollback Plan

**Triggers** (когда откатывать):
-
-

**Steps** (шаги отката):
1.
2.

**Blast Radius**: {масштаб влияния отката}

### Refresh Triggers (когда пере-оценить)
-
-

---

## Affected Files

| File | Baseline Hash |
|------|---------------|
| | |

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-{NNN} | based_on |
| SOL-{NNN} | based_on |
