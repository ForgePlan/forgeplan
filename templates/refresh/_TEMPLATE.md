# REF-{NNN}: Refresh — {Original Artifact ID}

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | YYYY-MM-DD |
| Original | {ADR-NNN / PRD-NNN — артефакт который пере-оцениваем} |
| Reason | expired / context_changed / new_evidence / drift_detected |

## Original Decision Summary

{Краткое резюме исходного решения}

## What Changed

{Что изменилось с момента исходного решения:
- Новые данные / evidence
- Изменение контекста
- Технологический drift
- Истечение valid_until}

## Re-evaluation

### Still Valid?

{Да / Нет / Частично — с обоснованием}

### New Evidence

| Evidence | Verdict | CL | Score |
|----------|---------|-----|-------|
| | supports/weakens/refutes | 0-3 | |

### Updated R_eff

{Новый R_eff score после пере-оценки}

## Recommendation

{Одно из:
- **Reaffirm**: решение всё ещё верно, обновить valid_until
- **Modify**: частичные изменения, создать новый ADR
- **Supersede**: полная замена, создать новый ADR с supersedes link
- **Deprecate**: решение больше не актуально}

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| {original} | refreshes |
