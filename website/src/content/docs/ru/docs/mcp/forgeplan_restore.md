---
title: forgeplan_restore
description: "Restore a soft-deleted artifact from the most recent non-consumed receipt in the trash."
---

Отменяет delete / supersede / deprecate по последнему подходящему,
неиспользованному receipt в `.forgeplan/trash/`. Восстанавливает канонический
Markdown, статус и доступные связи; в legacy также восстанавливает строку
проекции, а в мигрированном workspace отдельно сообщает reconciliation.
Конфликт identity может остановить восстановление. Срок receipt по умолчанию
30 дней; операция не заменяет отдельную резервную копию.

**Категория**: Lifecycle / Recovery

## Когда агент вызывает

- «Восстанови PRD-042» - пользователь заметил, что вчера был удалён не тот артефакт.
- После `forgeplan_supersede`, который агент осознаёт как ошибочный: восстановить
  оригинал.
- Точечное восстановление, когда [`forgeplan_undo_last`](/ru/docs/mcp/forgeplan_undo_last/)
  откатил бы не ту операцию - передача ID точнее, чем «самое последнее».
- После отдельного изучения receipt: этот инструмент выполняет восстановление и может удалять просроченные receipts; read-only preview параметра нет.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта для восстановления из самого свежего не-consumed receipt. |
| `target_status` | `string \| null` | no | Вместо прежнего статуса receipt задать `draft`, `active` или `deprecated`; гейты переходов сохраняются. |
| `workspace` | `string \| null` | no | Явный абсолютный путь workspace или путь с `~/`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::RestoreParams`_

## Возвращает

Примеры успеха ниже иллюстрируют поля восстановления, не зафиксированные
измерения. Возможны дополнительные поля восстановленного статуса
и публикации индекса. В мигрированном workspace проверяйте `index_pending`.

```json
{
  "restored": "PRD-042",
  "op_reversed": "delete",
  "relations_restored": 3,
  "relations_skipped": [],
  "projection_restored": true,
  "warnings": [],
  "_next_action": "Restored `PRD-042` (reversed delete). 3 relation(s) restored. Verify with `forgeplan_get PRD-042`."
}
```

Когда некоторых целей связей больше нет:

```json
{
  "restored": "PRD-042",
  "op_reversed": "delete",
  "relations_restored": 2,
  "relations_skipped": ["EVID-099", "RFC-007"],
  "projection_restored": true,
  "warnings": [],
  "_next_action": "Restored `PRD-042` (reversed delete). 2 relation(s) restored, 2 skipped because targets no longer exist. Review with `forgeplan_get PRD-042` and re-link manually if needed."
}
```

Отсутствие подходящего receipt возвращает MCP tool error с поясняющим
текстом и текстовой подсказкой `_next_action`, не прежний JSON-envelope
`ok`/`error`. Для недавних операций вызовите `forgeplan_activity` с
`{"tool":"forgeplan_delete,forgeplan_supersede,forgeplan_deprecate","since_hours":720}`.
Этот запрос журнала сам ничего не восстанавливает.

## Пример вызова

```json
{ "id": "PRD-042" }
```

## Типичная последовательность

1. [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) - найти, когда произошла деструктивная операция.
2. `forgeplan_restore` - восстановить конкретный артефакт.
3. [`forgeplan_get`](/ru/docs/mcp/forgeplan_get/) - проверить тело, статус, связи.
4. При необходимости - вручную перевосстановить пропущенные связи.

## CLI эквивалент

[`forgeplan restore <id>`](/ru/docs/cli/) - та же семантика восстановления.

## См. также

- [`forgeplan_undo_last`](/ru/docs/mcp/forgeplan_undo_last/) - откатить самую последнюю деструктивную операцию (без указания ID)
- [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) - найти receipt перед восстановлением
- [`forgeplan_delete`](/ru/docs/mcp/forgeplan_delete/) - мягкое удаление, которое это откатывает
- [Обзор MCP](/ru/docs/mcp/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Restore a soft-deleted artifact from the most recent non-consumed receipt in `.forgeplan/trash/`. Works for delete (recreates row + moves projection back), supersede (resets status + drops link), and deprecate (resets status). Refuses if a different artifact with the same ID currently exists (manual resolution required). TTL default: 30 days from the destructive op.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | да | string | Artifact ID to recover from the most recent non-consumed receipt. |
| `target_status` | нет | string \| null | Issue #291: optional explicit override of the restored status.<br>Default behaviour reads `prior_status` from the receipt snapshot<br>(which captures the artifact's status at the moment of `_delete`<br>/ `_deprecate` / `_supersede`). Pass `target_status: "active"` to<br>force a specific status — useful when the operator wants to<br>restore to a different state than was captured.<br><br>Valid values: `"draft"`, `"active"`, `"deprecated"`.<br>Unknown values are rejected with a structured error. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to recover from the most recent non-consumed receipt.",
      "type": "string"
    },
    "target_status": {
      "default": null,
      "description": "Issue #291: optional explicit override of the restored status.\nDefault behaviour reads `prior_status` from the receipt snapshot\n(which captures the artifact's status at the moment of `_delete`\n/ `_deprecate` / `_supersede`). Pass `target_status: \"active\"` to\nforce a specific status — useful when the operator wants to\nrestore to a different state than was captured.\n\nValid values: `\"draft\"`, `\"active\"`, `\"deprecated\"`.\nUnknown values are rejected with a structured error.",
      "type": [
        "string",
        "null"
      ]
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
  "title": "RestoreParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
