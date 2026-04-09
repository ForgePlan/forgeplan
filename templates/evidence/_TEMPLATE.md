# EVID-{NNN}: {Evidence Title}

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | YYYY-MM-DD |
| Valid Until | YYYY-MM-DD |
| Target | ADR-{NNN} (решение которое подтверждаем/опровергаем) |

<!-- REQUIRED for R_eff scoring. Legal values documented in templates/evidence/README.md. -->

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

{Что измерено, как измерено, в каких условиях}

## Result

{Конкретный результат с числами}

## Interpretation

{Что результат означает для целевого решения}

## Congruence Level Justification

<!-- Legend: CL3 same-context (penalty 0.0); CL2 related (0.1); CL1 external (0.4); CL0 opposed (0.9). -->

{Обоснование выбранного Congruence Level}

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-{NNN} | informs |
