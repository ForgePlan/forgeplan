---
title: forgeplan_constellation_search
description: "forgeplan_constellation_search — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

В выбранной локальной реализации этот full-profile инструмент сначала
пытается выполнить кэшированный pinned semantic search; при недоступности
или ошибке semantic может ответить лексически. Проверяйте `mode`,
`semantic_available`, `semantic_unavailable` или `semantic_error`,
а также `eligible_rows`, если поле возвращено. Описание lexical-only/later-ANN
в захваченном каталоге ниже отстаёт от handler. Запрос не скачивает веса.
Лимит по умолчанию 10, ограничивается 1..=100. Это не `forgeplan_search`
и не handle-based `constellation_search` read-профиля.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Lexical search over the pinned Constellation artifacts table —                        case-insensitive substring match on canonical_key, title and body                        at the *published* generation (a pinned reader, never latest).                        Each hit reports its stored `semantic_state` honestly; ANN/vector                        search is a later stage and is not claimed here.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `limit` | нет | integer \| null | Max hits (default 10, hard cap 100) |
| `query` | да | string | Substring to match (case-insensitive) against canonical_key, title, body |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call — same routing as<br>every other read tool (PRD-078 / ADR-015). |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "limit": {
      "default": null,
      "description": "Max hits (default 10, hard cap 100)",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "query": {
      "description": "Substring to match (case-insensitive) against canonical_key, title, body",
      "type": "string"
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call — same routing as\nevery other read tool (PRD-078 / ADR-015).",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "query"
  ],
  "title": "ConstellationSearchParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->

## Примеры

- [Настройка MCP и область workspace](/ru/docs/guides/mcp-setup/)
- [Сценарий Constellation и последовательность handles](/ru/docs/guides/constellation/)

## См. также

- [MCP reference](/ru/docs/mcp/)
- [Constellation reference](/ru/docs/reference/constellation/)
