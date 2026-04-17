---
depth: tactical
id: PROB-038
kind: problem
links:
- target: EPIC-004
  relation: informs
status: draft
title: Validator false-positive on tech names in HTML template comments (prd-no-impl-leakage)
---

# PROB-038: Validator false-positive — tech names в HTML comments

## Signal

`forgeplan validate PRD-*` на всех 5 новых PRD (049-053) выдаёт SHOULD warning:

```
! [SHOULD] prd-no-impl-leakage: Tech names in FR/NFR sections:
  aws, django, docker, postgresql, react, redis, rest
```

Эти слова находятся в **HTML-комментариях template'а** `<!-- -->`, содержащих
guidance: "НЕ использовать этих технологий в FR/NFR". Validator читает
HTML-комментарии как содержимое → ложное срабатывание.

## Constraints

- Validator must still catch **real** tech leakage (React/Django/etc в реальном
  FR тексте) — ослабить правило нельзя.
- HTML-комментарии в markdown — стандартный template guidance; комменты нельзя
  удалить без потери ценности шаблона.
- Backward compat — старые PRD не должны re-validate с новыми результатами.

## Optimization Targets

- **Убрать false positives** — 5 PRD с SHOULD warning'ами без реального нарушения.
- **Сохранить detection** — реальные tech names в non-comment FR/NFR секциях
  продолжают триггериться.

## Observation Indicators (Anti-Goodhart)

- НЕ оптимизировать только под HTML-комменты — pattern должен быть robust
  (код-fences в markdown, quoted strings).
- НЕ убирать правило целиком — real leakage check нужен.

## Acceptance Criteria

Validator strip'ает markdown comments перед tech-name анализом:
1. `<!-- ... -->` (одна строка или multi-line)
2. ` ``` ... ``` ` code fences (опционально)
3. Result: PRD-049..053 validate → 0 SHOULD warnings по `prd-no-impl-leakage`

## Blast Radius

- Файл: `crates/forgeplan-core/src/validation/` (tech leakage rule).
- Не затрагивает: лексер markdown, scoring, CLI surface.
- Регрессионный риск: low.

## Reversibility

**High** — чистый validator fix, rollback = revert commit.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| EPIC-004 | informs |
| PROB-034 | informs |
