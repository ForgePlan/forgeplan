---
title: constellation_get
description: "constellation_get — constellation-read MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `constellation-read`.

Fetch one artifact by canonical key or display id. A `key@fragment`                        qualifier narrows the scope first; an unqualified id claimed by                        two sources answers `ambiguous` with the claimers.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `id` | да | string | Canonical key or display id; a `key@source-fragment` qualifier<br>narrows the scope before reading. |
| `scope_handle` | нет | string \| null |  |
| `snapshot_handle` | нет | string \| null |  |
| `strict` | нет | boolean \| null | Strict reads (default) fail with `source_unavailable` when a<br>required source cannot answer; `false` accepts a partial answer. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Canonical key or display id; a `key@source-fragment` qualifier\nnarrows the scope before reading.",
      "type": "string"
    },
    "scope_handle": {
      "type": [
        "string",
        "null"
      ]
    },
    "snapshot_handle": {
      "type": [
        "string",
        "null"
      ]
    },
    "strict": {
      "description": "Strict reads (default) fail with `source_unavailable` when a\nrequired source cannot answer; `false` accepts a partial answer.",
      "type": [
        "boolean",
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

## Примеры

- [Настройка MCP и область workspace](/ru/docs/guides/mcp-setup/)
- [Сценарий Constellation и последовательность handles](/ru/docs/guides/constellation/)

## См. также

- [MCP reference](/ru/docs/mcp/)
- [Constellation reference](/ru/docs/reference/constellation/)
