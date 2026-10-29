---
title: constellation_impact
description: "constellation_impact — constellation-read MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `constellation-read`.

Explainable impact: who depends on the artifact, by which                        relation path, and where the bounded walk stopped. `based_on`                        and friends propagate transitively; `references`/`informs`                        only mark the direct neighbor.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `depth` | нет | integer \| null | Maximum structural hops (default 4). |
| `id` | да | string | The changed artifact's canonical key or display id. |
| `max_nodes` | нет | integer \| null | Maximum impacted nodes (default 256). |
| `scope_handle` | нет | string \| null |  |
| `snapshot_handle` | нет | string \| null |  |
| `strict` | нет | boolean \| null | Strict reads (default) fail with `source_unavailable` when a<br>required source cannot answer; `false` accepts a partial answer. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "depth": {
      "description": "Maximum structural hops (default 4).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "id": {
      "description": "The changed artifact's canonical key or display id.",
      "type": "string"
    },
    "max_nodes": {
      "description": "Maximum impacted nodes (default 256).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
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
  "title": "ImpactParams",
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
