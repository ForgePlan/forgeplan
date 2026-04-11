---
title: forgeplan_review
description: "Проверить артефакт — запустить валидацию и показать контрольный список жизненного цикла. Показывает результаты MUST/SHOULD и готовность артефакта к активации."
---

Проверяет один артефакт — запускает валидацию (правила MUST / SHOULD с учётом глубины), проверяет предварительные условия жизненного цикла (доказательство, R_eff, связанные отношения) и возвращает чёткий вердикт о том, готов ли артефакт к активации. Это объединённый гейт "могу ли я активировать?", используемый перед вызовом `forgeplan_activate`.

**Категория**: Качество

## Когда агент вызывает его

- **Перед активацией** — убедитесь, что все гейты пройдены, чтобы избежать ошибок активации.
- **Ревью PR** — запускается для каждого затронутого артефакта, чтобы выявить отсутствующие разделы или сбои MUST.
- **Самопроверка автора** — быстрее, чем запуск `validate` + `score` + ручная проверка жизненного цикла.
- **Автоматизированные хуки качества** — pre-commit / CI может вызвать это и прервать сборку, если какой-либо артефакт регрессирует.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | да | ID артефакта для проверки. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ReviewParams`_

## Возвращает

```json
{
  "artifact_id": "PRD-042",
  "kind": "prd",
  "depth": "standard",
  "status": "draft",
  "validation": {
    "must_errors": [],
    "should_warnings": ["density < 50 words in section Goals"]
  },
  "lifecycle": {
    "r_eff": 0.72,
    "has_evidence": true,
    "ready_to_activate": true
  },
  "verdict": "ПРОЙДЕНО — готов к активации"
}
```

Если заблокировано:

```json
{
  "verdict": "FAIL",
  "validation": { "must_errors": ["Missing section: Problem"] },
  "lifecycle": { "ready_to_activate": false }
}
```

## Пример вызова

```json
{ "id": "PRD-042" }
```

## Типичная последовательность

1.  `forgeplan_review` → если `FAIL`, исправьте проблемы.
2.  `forgeplan_update` для исправления тела.
3.  `forgeplan_review` снова → ожидается `PASS`.
4.  `forgeplan_activate` — перевести черновик → активный.

## Эквивалент CLI

```bash
forgeplan review PRD-042
```

## См. также

- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) — только валидация, без проверки жизненного цикла.
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) — только пересчёт R_eff.
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) — действие, контролируемое этим ревью.
- [Руководство по методологии](/docs/methodology/overview/)