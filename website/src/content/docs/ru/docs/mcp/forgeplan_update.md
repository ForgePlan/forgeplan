---
title: forgeplan_update
description: "Обновляет метаданные артефакта (статус, заголовок) и/или тело. Перерисовывает проекцию markdown после обновления."
---

Применяет изменения к телу и/или метаданным frontmatter существующего артефакта на месте. Это путь прямого редактирования - агент использует его для заполнения только что созданной заглушки, переписывания раздела после обратной связи от пользователя или переименования артефакта. Для переходов жизненного цикла (draft → active → superseded) агент должен использовать специализированные инструменты `forgeplan_activate` / `forgeplan_supersede` / `forgeplan_deprecate` вместо этого, которые обеспечивают гейты валидации.

**Категория**: Редактирование артефактов

## Когда агент вызывает это

- Сразу после `forgeplan_new` для заполнения ОБЯЗАТЕЛЬНЫХ разделов (Problem, Goals, Non-Goals, FR).
- Применение запрошенного пользователем изменения: "добавить раздел о наблюдаемости в RFC-007".
- Исправление обнаружения валидатора: `forgeplan_validate` сообщает об отсутствующем разделе, агент записывает его и повторно валидирует.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | yes | ID артефакта для обновления. |
| `status` | `string` или `null` | no | Схема перечисляет `draft`, `active`, `superseded`, `deprecated`, но обработчик разрешает прямую запись только `draft`. Для остальных значений используйте инструменты жизненного цикла. |
| `title` | `string` | no | Новый заголовок. |
| `body` | `string` | no | Новое содержимое тела (полная замена markdown). |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::UpdateParams`_

## Возвращает

Возвращает `id`, `message`, `status`, `title`, трассировку workspace и сведения о согласовании индекса. Сначала обновляется канонический Markdown; `index_pending` отдельно сообщает о производном индексе. Запись статусов `active`, `superseded`, `deprecated` отклоняется, хотя схема их перечисляет. Если все поля обновления отсутствуют или равны `null`, возвращается “Nothing to update.”

Учебный фрагмент ответа:

```json
{
  "id": "PRD-042",
  "message": "Updated successfully",
  "status": "draft",
  "title": "Система аутентификации и сессий"
}
```

## Пример вызова

```json
{ "id": "PRD-042", "title": "Authentication and session system" }
```

В типичном контексте агента:

> Агент только что запустил `forgeplan_new`, получил `PRD-042`, и теперь заполняет ОБЯЗАТЕЛЬНЫЕ разделы одной заменой тела.

```json
{ "id": "PRD-042", "body": "# PRD-042: Authentication system\n\n## Problem\n..." }
```

## Типичная последовательность

`forgeplan_new` → `forgeplan_update` (заполнение заглушки) → `forgeplan_validate` → цикл по любым обнаружениям → `forgeplan_activate`. Для поздних изменений активного артефакта шаблон такой: `forgeplan_get` → вычислить патч → `forgeplan_update`.

## Эквивалент CLI

- [`forgeplan update`](/ru/docs/cli/update/), например `forgeplan update PRD-042 --title "Система аутентификации и сессий"` или `forgeplan update PRD-042 --body @body.md`. CLI также предоставляет `--depth`, которого нет в MCP-схеме.

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_activate`](/ru/docs/mcp/forgeplan_activate/) - правильный путь для `draft → active`
- [`forgeplan_validate`](/ru/docs/mcp/forgeplan_validate/) - валидировать после редактирования
- [`forgeplan_supersede`](/ru/docs/mcp/forgeplan_supersede/) - заместить вместо редактирования

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Update artifact metadata (status, title) and/or body. Re-renders markdown projection after update.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `body` | нет | string \| null | New body content |
| `id` | да | string | Artifact ID to update |
| `status` | нет | StatusKind \| null | New status |
| `title` | нет | string \| null | New title |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "StatusKind": {
      "enum": [
        "draft",
        "active",
        "superseded",
        "deprecated"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "body": {
      "default": null,
      "description": "New body content",
      "type": [
        "string",
        "null"
      ]
    },
    "id": {
      "description": "Artifact ID to update",
      "type": "string"
    },
    "status": {
      "anyOf": [
        {
          "$ref": "#/$defs/StatusKind"
        },
        {
          "type": "null"
        }
      ],
      "description": "New status"
    },
    "title": {
      "default": null,
      "description": "New title",
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
  "title": "UpdateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
