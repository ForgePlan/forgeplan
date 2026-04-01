# EVID-{NNN}: {Evidence Title}

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | YYYY-MM-DD |
| Valid Until | YYYY-MM-DD |
| Target | ADR-{NNN} (решение которое подтверждаем/опровергаем) |

<!-- Fill in the Structured Fields section below for R_eff scoring.
     These fields are REQUIRED for correct R_eff calculation.
     evidence_type: measurement | test | benchmark | audit
     verdict: supports | weakens | refutes
     congruence_level: 0 | 1 | 2 | 3 (CL3=same context, CL0=opposed context)
-->

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

<!-- Почему выбран именно этот CL:
     CL3: тот же контекст, внутренний тест (penalty 0.0)
     CL2: похожий контекст, related project (penalty 0.1)
     CL1: другой контекст, внешняя документация (penalty 0.4)
     CL0: противоположный контекст (penalty 0.9) -->

{Обоснование выбранного Congruence Level}

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-{NNN} | informs |
