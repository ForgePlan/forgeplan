---
title: constellation_search
description: "constellation_search — constellation-read MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

Нужен пригодный `scope_handle` или `snapshot_handle` из текущего процесса сервера, хотя общая схема помечает оба необязательными. Чтение live-scope истекает при изменении зафиксированных байтов источника: откройте scope заново, не переиспользуйте старую идентичность. Snapshot сообщает о пропущенных/недоступных источниках, не читая их незаметно в live-режиме.

Это лексический, не семантический поиск. Видимый источник не доказывает, что его строки имеют эмбеддинги и допущены к семантическому индексу. Снимок интерфейса относится к выбранной невыпущенной сборке `0.39.0+constellation.s1.1`, а не к неизменённому публичному/глобальному `0.38.0`.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `constellation-read`.

Lexical search across the scope's live sources or a snapshot's                        pinned revisions. Each hit names its source and the version it                        was read at.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `limit` | нет | integer \| null | Maximum hits (default 10). |
| `query` | да | string |  |
| `scope_handle` | нет | string \| null |  |
| `snapshot_handle` | нет | string \| null |  |
| `strict` | нет | boolean \| null | Strict reads (default) fail with `source_unavailable` when a<br>required source cannot answer; `false` accepts a partial answer.<br>Ignored for snapshot reads — the lockfile's source set is frozen. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "limit": {
      "description": "Maximum hits (default 10).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "query": {
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
      "description": "Strict reads (default) fail with `source_unavailable` when a\nrequired source cannot answer; `false` accepts a partial answer.\nIgnored for snapshot reads — the lockfile's source set is frozen.",
      "type": [
        "boolean",
        "null"
      ]
    }
  },
  "required": [
    "query"
  ],
  "title": "SearchParams",
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
