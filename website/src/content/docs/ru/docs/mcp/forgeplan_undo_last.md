---
title: forgeplan_undo_last
description: "Reverse the most recent destructive operation (delete / supersede / deprecate) - undo button for AI agents."
---

Идёт по trash от свежего к старому, находит самый свежий не-consumed receipt в
пределах `within_hours` и применяет ту же логику восстановления, что и
[`forgeplan_restore`](/ru/docs/mcp/forgeplan_restore/). Используйте, когда агент
понимает «последнее, что я сделал, было неправильным», без необходимости знать ID
артефакта. Возвращает ошибку с подсказкой, если подходящего receipt нет - никогда
не угадывает.

**Категория**: Lifecycle / Recovery

## Когда агент вызывает

- Сразу после ошибочного `forgeplan_delete` / `_supersede` / `_deprecate`.
- Пользователь говорит «отмени это», не указывая, какой артефакт.
- Восстановление после галлюцинации LLM, которая совершила деструктивное действие.
- В паре с [`forgeplan_activity_stats`](/ru/docs/mcp/forgeplan_activity_stats/) -
  увидели неожиданный деструктивный вызов, откатываете его.

## Входные параметры

| Имя | Тип | Обязательно | Описание |
|---|---|---|---|
| `within_hours` | `integer` или `null` | no (по умолчанию 24) | Без параметра или при null — 24 часа. Обработчик ограничивает переданное целое диапазоном 1–720; схема показывает minimum 0, но не это ограничение или верхнюю границу. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::UndoLastParams`_

## Возвращает

Учебные фрагменты, не запись выполненного здесь восстановления. Успешный ответ может также содержать трассировку workspace и поля согласования индекса.

```json
{
  "restored": "PRD-042",
  "op_reversed": "delete",
  "receipt_id": "trash-2026-04-26T10-14-22-001",
  "relations_restored": 3,
  "relations_skipped": [],
  "projection_restored": true,
  "warnings": [],
  "_next_action": "Reversed most recent delete of `PRD-042`. To undo another, call `forgeplan_undo_last` again (finds the next newest non-consumed receipt). Or restore a specific ID: `forgeplan_restore <id>`."
}
```

Когда в окне нечего откатывать:

Обработчик возвращает ошибочный MCP `CallToolResult` с текстом, а не JSON-объектом `ok: false`. Пример начала текста:

```text
No non-consumed destructive op in the last 24 hour(s).

_next_action: Expand the window: `forgeplan_undo_last within_hours=720`.
```

## Пример вызова

Дефолтное окно 24 ч:

```json
{}
```

Более широкий поиск после периода простоя:

```json
{ "within_hours": 720 }
```

## Типичная последовательность

1. Происходит осечка (`forgeplan_delete`, `_supersede` или `_deprecate`).
2. `forgeplan_undo_last` - откатить.
3. Повторить вызов, чтобы откатить предыдущую операцию (каждый вызов потребляет
   самый свежий не-consumed receipt).
4. Или переключиться на [`forgeplan_restore <id>`](/ru/docs/mcp/forgeplan_restore/),
   когда конкретный ID известен.

## CLI эквивалент

[`forgeplan undo-last`](/ru/docs/cli/) — та же операция выбора записи восстановления. Команда `forgeplan undo` в выбранном CLI не зарегистрирована.

## См. также

- [`forgeplan_restore`](/ru/docs/mcp/forgeplan_restore/) - восстановить конкретный артефакт по ID
- [`forgeplan_activity`](/ru/docs/mcp/forgeplan_activity/) - изучить таймлайн деструктивных операций
- [`forgeplan_delete`](/ru/docs/mcp/forgeplan_delete/) - мягкое удаление, которое это откатывает
- [Обзор MCP](/ru/docs/mcp/)

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Reverse the most recent destructive operation (delete, supersede, or deprecate) by reading the soft-delete trash and applying forgeplan_restore to the most recently written non-consumed receipt. Params: within_hours (default 24, max 720). If no matching receipt is found, returns an error with guidance; the tool never guesses.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `within_hours` | нет | integer \| null | Time window (hours) to search for the last destructive op.<br>Default 24, max 720 (30 days). |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "within_hours": {
      "default": null,
      "description": "Time window (hours) to search for the last destructive op.\nDefault 24, max 720 (30 days).",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
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
  "title": "UndoLastParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
