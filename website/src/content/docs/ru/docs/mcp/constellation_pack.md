---
title: constellation_pack
description: "constellation_pack — constellation-read MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

Хотя оба ключа handle необязательны в схеме, рабочему вызову нужен
действующий `scope_handle` или `snapshot_handle`. Оценщик выбранной
реализации считает `ceil(UTF-8 bytes / 4)`, несмотря на сокращение
“chars/4” в каталоге. Это не токенизация модели и не жёсткая граница
сериализованного ответа: бюджет единиц контента не включает envelope.
Успех для live scope содержит `pack` и `payload_bytes`;
успех для snapshot возвращает pack напрямую.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `constellation-read`.

Build a budgeted ContextPack for a task over the scope's live                        sources or a snapshot's pinned revisions: lexically ranked                        evidence units with roles, named omissions and a spend receipt.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `budget` | нет | integer \| null | Token budget (estimated tokens, chars/4; default 8000). |
| `scope_handle` | нет | string \| null |  |
| `snapshot_handle` | нет | string \| null |  |
| `strict` | нет | boolean \| null | Strict reads (default) fail with `source_unavailable` when a<br>required source cannot answer; `false` accepts a partial answer. |
| `task` | да | string | What the agent must do — ranking terms come from this text. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "budget": {
      "description": "Token budget (estimated tokens, chars/4; default 8000).",
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
    },
    "task": {
      "description": "What the agent must do — ranking terms come from this text.",
      "type": "string"
    }
  },
  "required": [
    "task"
  ],
  "title": "PackParams",
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
