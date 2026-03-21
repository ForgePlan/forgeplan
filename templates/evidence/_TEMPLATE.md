# EVID-{NNN}: {Evidence Title}

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | YYYY-MM-DD |
| Valid Until | YYYY-MM-DD |
| Type | measurement / test / benchmark / audit |
| Verdict | supports / weakens / refutes |
| CL | 0 / 1 / 2 / 3 |
| Target | ADR-{NNN} (решение которое подтверждаем/опровергаем) |

## Measurement

{Что измерено, как измерено, в каких условиях}

## Result

{Конкретный результат с числами}

## Interpretation

{Что результат означает для целевого решения}

## Congruence Level Justification

{Почему выбран именно этот CL:
- CL3: тот же контекст, внутренний тест (penalty 0.0)
- CL2: похожий контекст, related project (penalty 0.1)
- CL1: другой контекст, внешняя документация (penalty 0.4)
- CL0: противоположный контекст (penalty 0.9)}

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-{NNN} | informs |
