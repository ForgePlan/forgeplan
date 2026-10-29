---
title: forgeplan_list
description: "Краткий список артефактов с необязательными фильтрами по типу и статусу."
---

Возвращает отфильтрованный перечень артефактов в рабочем пространстве. Это самый экономичный вызов для обнаружения - агенты используют его для ответов на вопросы, такие как "какие PRD активны?" или "сколько у нас просроченных ADR?" без получения полного содержимого. Для полнотекстового обнаружения агенту следует использовать `forgeplan_search`.

**Категория**: Чтение артефактов

## Когда агент вызывает эту функцию

- Запуск сессии: агенту нужен быстрый обзор существующих элементов, прежде чем принимать решение о следующем действии.
- Фильтрация по состоянию: "покажи мне все черновики PRD" перед выполнением пакетной валидации.
- Ответ на вопрос пользователя, например "сколько RFC мы выпустили?" - полное содержимое не требуется.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `kind` | `string` | no | Фильтр по типу (`prd`, `rfc`, `adr`, ...). |
| `status` | `string` | no | Фильтр по статусу (`draft`, `active`, `stale`, `superseded`, `deprecated`). |
| `workspace` | `string \| null` | no | Явный абсолютный путь workspace или путь с `~/`. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::ListParams`_

## Возвращает

JSON-текст payload — объект с `artifacts`, `total`, `_next_action`,
не массив верхнего уровня и не поле `count`. У краткой записи есть
`id`, `kind`, `status`, `title`, `id_canonical`, `id_display`, необязательные
`slug`, `predicted_number`, `assigned_number`. Полей `depth`, `tags`,
`updated_at`, `body` нет; полное содержимое читайте через `forgeplan_get`.
MCP-фильтра `tag` нет. Nullable-поля `kind`, `status`, `workspace`
необязательны, а не требуют передачи ключа со значением null.

## Пример вызова

```json
{ "kind": "prd", "status": "active" }
```

В типичном контексте агента:

> Агент начинает сессию; хочет получить быстрый снимок активной работы, прежде чем читать какое-либо содержимое.

```json
{ "status": "active" }
```

## Типичная последовательность

Обычно это второй или третий вызов сессии - после `forgeplan_init`/`forgeplan_health` и перед `forgeplan_get` для конкретного ID. Агенты часто передают результат в сводку для пользователя без каких-либо дальнейших вызовов MCP.

## Эквивалент CLI

- [`forgeplan list`](/ru/docs/cli/list/) - те же фильтры, вывод в виде удобочитаемой таблицы

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_get`](/ru/docs/mcp/forgeplan_get/) - читать полное содержимое одного артефакта
- [`forgeplan_search`](/ru/docs/mcp/forgeplan_search/) - полнотекстовое / семантическое обнаружение
- [`forgeplan_health`](/ru/docs/mcp/forgeplan_health/) - агрегированное состояние проекта

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

List artifacts with optional kind/status filters. Returns ID, kind, status, and title for each artifact.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `kind` | нет | string \| null | Filter by kind (optional) |
| `status` | нет | string \| null | Filter by status (optional) |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "kind": {
      "default": null,
      "description": "Filter by kind (optional)",
      "type": [
        "string",
        "null"
      ]
    },
    "status": {
      "default": null,
      "description": "Filter by status (optional)",
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
  "title": "ListParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
