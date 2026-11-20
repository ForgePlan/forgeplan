---
title: forgeplan_review
description: "Проверить артефакт - запустить валидацию и показать контрольный список жизненного цикла. Показывает результаты MUST/SHOULD и готовность артефакта к активации."
---

Проверяет результаты depth-aware валидации и условия жизненного цикла, возвращает `can_activate` и предупреждения. Это read-only предварительная проверка, не активация и не гарантия успешной последующей записи: handler активации отдельно проверяет настроенный provenance-гейт и условия на момент записи.

**Категория**: Качество

## Когда агент вызывает его

- **Перед активацией** - убедитесь, что все гейты пройдены, чтобы избежать ошибок активации.
- **Ревью PR** - запускается для каждого затронутого артефакта, чтобы выявить отсутствующие разделы или сбои MUST.
- **Самопроверка автора** - быстрее, чем запуск `validate` + `score` + ручная проверка жизненного цикла.
- **Автоматизированные хуки качества** - pre-commit / CI может вызвать это и прервать сборку, если какой-либо артефакт регрессирует.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта для проверки. |
| `workspace` | `string \| null` | no | Явный абсолютный путь workspace или путь с `~/`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::ReviewParams`_

## Возвращает

JSON-текст payload содержит `artifact_id`, boolean `can_activate`,
`must_findings`, `should_findings`, `warnings`, `_next_action`.
Вложенных объектов `validation`/`lifecycle`, числового `r_eff`
и строкового поля `verdict` в ответе нет. `can_activate: false` —
результат обзора; tool error отдельно означает невозможность выполнить обзор.

## Пример вызова

```json
{ "id": "PRD-042" }
```

## Типичная последовательность

1. Изучите `can_activate`, все findings и предупреждения `forgeplan_review`.
2. Исправьте подтверждённые проблемы через `forgeplan_update`; не придумывайте правки ради чистого отчёта.
3. Повторите review, сохранив невыполненные обязательства по реализации и evidence.
4. Вызывайте `forgeplan_activate` лишь после выполнения условий задачи; изучите отдельный результат активации.

## Эквивалент CLI

```bash
forgeplan review PRD-042
```

## См. также

- [`forgeplan_validate`](/ru/docs/mcp/forgeplan_validate/) - только валидация, без проверки жизненного цикла.
- [`forgeplan_score`](/ru/docs/mcp/forgeplan_score/) - только пересчёт R_eff.
- [`forgeplan_activate`](/ru/docs/mcp/forgeplan_activate/) - действие, контролируемое этим ревью.
- [Руководство по методологии](/ru/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Review an artifact — run validation and show lifecycle checklist. Shows MUST/SHOULD findings and whether artifact can be activated.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | да | string | Artifact ID to review |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to review",
      "type": "string"
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "ReviewParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
