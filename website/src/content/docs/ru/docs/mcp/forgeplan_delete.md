---
title: forgeplan_delete
description: "Мягко удаляет артефакт с записью восстановления и переносит Markdown-проекцию в trash."
---

Мягко удаляет артефакт из рабочего пространства. Сервер сохраняет запись восстановления и переносит канонический Markdown в `.forgeplan/trash/`; производную строку удаляет только при legacy-владельце индекса. Восстановление через `forgeplan_undo_last` или `forgeplan_restore` зависит от доступности записи, срока действия и проверок коллизий. Для вывода решений из эксплуатации предпочитайте `supersede` или `deprecate`. Вызывайте `forgeplan_delete` только по явной просьбе пользователя удалить опечатку, тестовый артефакт или дубликат.

**Категория**: Редактирование артефактов

## Когда агент вызывает это

- Пользователь явно говорит: «удалить NOTE-099, я создал его по ошибке».
- Очистка тестового артефакта из запусков `forge-smoke`.
- Удаление сироты (артефакт без связей), созданного во время прерванного процесса декомпозиции.

Агентам НИКОГДА не следует вызывать это для «вывода из эксплуатации» активного решения - используйте `forgeplan_supersede` или `forgeplan_deprecate`, чтобы сохранить историю.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта для удаления. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::DeleteParams`_

## Возвращает

Ответ содержит ID артефакта и записи восстановления. Обработчик сохраняет связи для восстановления, но не реализует ранее описанный отказ из-за входящих зависимостей. В migrated workspace проверяйте поля согласования индекса, например `index_pending`, отдельно от канонического удаления.

Учебный фрагмент ответа:

```json
{
  "id": "NOTE-099",
  "title": "Тестовая заметка",
  "message": "Soft-deleted — recoverable via forgeplan_undo_last",
  "receipt_id": "illustrative-receipt-id"
}
```

## Пример вызова

```json
{ "id": "NOTE-099" }
```

В типичном контексте агента:

> Пользователь говорит: «PROB-099 был ошибкой, уничтожь его». Агент подтверждает с пользователем, затем удаляет.

```json
{ "id": "PROB-099" }
```

## Типичная последовательность

`forgeplan_list` или `forgeplan_get` для подтверждения цели → явное подтверждение пользователя → `forgeplan_delete` → `forgeplan_health` для проверки отсутствия сбоев. Для обратимого вывода из эксплуатации вместо этого используется `forgeplan_deprecate` (active → deprecated).

## Эквивалент CLI

- [`forgeplan delete`](/ru/docs/cli/delete/) - та же операция, запрашивает подтверждение

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_deprecate`](/ru/docs/mcp/forgeplan_deprecate/) - обратимый вывод из эксплуатации
- [`forgeplan_supersede`](/ru/docs/mcp/forgeplan_supersede/) - замена с сохранением истории
- [`forgeplan_export`](/ru/docs/mcp/forgeplan_export/) - резервное копирование перед деструктивными операциями

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Delete an artifact: writes a soft-delete receipt, moves its markdown projection into `.forgeplan/trash/`, and drops the legacy projection row on workspaces whose index the legacy store still owns.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | да | string | Artifact ID to delete |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to delete",
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
  "title": "DeleteParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
