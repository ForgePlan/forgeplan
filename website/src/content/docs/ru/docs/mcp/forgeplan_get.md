---
title: forgeplan_get
description: "Считывает полный артефакт по ID. Возвращает все метаданные и содержимое тела."
---

Возвращает полное тело артефакта в формате Markdown, а также метаданные фронтматтера. Это каноническая операция "чтения" - агент вызывает её, когда ему требуется фактическое содержимое PRD/RFC/ADR, чтобы рассуждать о нём, цитировать или обновлять. В отличие от `forgeplan_list`, ответ содержит всё, что нужно агенту для понимания артефакта.

**Категория**: Чтение артефактов

## Когда агент вызывает эту операцию

- Пользователь спрашивает: "Что PRD-042 говорит об ограничениях скорости?" - агент извлекает, затем цитирует соответствующий раздел.
- Перед вызовом `forgeplan_update` - агенту требуется текущее тело, чтобы создать патч с учётом различий.
- Перед `forgeplan_reason` - агент предварительно считывает данные, чтобы описать контекст пользователю, если вызов LLM происходит медленно.

## Входные параметры

| Имя | Тип | Обязательный | Описание |
|---|---|---|---|
| `id` | `string` | yes | Display ID (`PRD-042`) или канонический slug (`prd-authentication-system`). |
| `workspace` | `string?` | no | Абсолютный путь или `~/`. Приоритет: запрос → `FORGEPLAN_WORKSPACE` → default сервера → поиск от CWD по родителям. |

_Источник схемы: `crates/forgeplan-mcp/src/server.rs::GetParams`_

## Возвращает

MCP `CallToolResult` содержит JSON-текст в `content[]`; разбирайте текстовый
payload. Успех - `ArtifactRecordDto` с `_next_action`, не envelope `artifact`:

| Поля | Форма |
|---|---|
| `id`, `kind`, `status`, `title`, `body`, `depth` | Строки |
| `author`, `parent_epic`, `valid_until` | Строки или null |
| `created_at`, `updated_at` | Строки; могут быть пустыми, если в захваченных метаданных нет даты |
| `r_eff_score` | Число |
| `slug`, `predicted_number`, `assigned_number` | Необязательные identity-поля |
| `id_canonical`, `id_display` | Строки канонической/display ссылки |
| `links` | `outbound` (`target`, `relation`) и `inbound` (`source`, `relation`); массивы могут быть пустыми |
| `_next_action` | Подсказка workflow |

Поля `tags` в DTO нет. Отсутствующий ID возвращает tool error.
Для мигрированных workspace локальной `0.39.0+constellation.s1.1` `body` -
полный канонический Markdown с frontmatter. Это отличается от full MCP
`forgeplan_constellation_get` (`key`, pinned `artifact.body` только с прозой)
и read-profile `constellation_get` (`id` и handle).
См. [справочник Constellation](/ru/docs/reference/constellation/).

## Пример вызова

```json
{ "id": "PRD-001", "workspace": "/path/to/project" }
```

С типичным контекстом агента:

> Пользователь спрашивает: "Напомни, что PRD-042 говорит об истечении срока действия токена". Агент извлекает полное тело, чтобы процитировать соответствующий раздел.

```json
{ "id": "PRD-042" }
```

## Типичная последовательность

`forgeplan_list` или `forgeplan_search` возвращает ID → `forgeplan_get` извлекает тело → агент цитирует / рассуждает / обновляет. Для массового чтения агентам следует предпочитать `forgeplan_search`, который возвращает ранжированные фрагменты, а не вызывать `get` в цикле.

## Эквивалент в CLI

- [`forgeplan get`](/ru/docs/cli/get/) - чтение в терминале, `--json` для машинного вывода

## См. также

- [Обзор MCP](/ru/docs/mcp/)
- [`forgeplan_list`](/ru/docs/mcp/forgeplan_list/) - сначала найдите ID
- [`forgeplan_update`](/ru/docs/mcp/forgeplan_update/) - редактирование после чтения
- [`forgeplan_search`](/ru/docs/mcp/forgeplan_search/) - обнаружение на уровне содержимого

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Read a full artifact by ID. Returns all metadata and body content.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | да | string | Artifact ID to read |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to read",
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
  "title": "GetParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
