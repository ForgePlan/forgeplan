---
title: constellation_graph
description: "constellation_graph — constellation-read MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

Хотя схема не требует ни одного handle, для содержательного вызова нужен действующий `scope_handle` от `constellation_open` либо `snapshot_handle` от `constellation_snapshot` в том же процессе сервера. Без обоих обработчик возвращает `unknown_handle`. Передавайте один вид handle за вызов. `limit` отдельно ограничивает число узлов и рёбер, но не гарантирует полноту графа.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `constellation-read`.

The relation graph of the scope's live sources or a snapshot's                        pinned revisions: every artifact as a source-qualified node,                        every declared link as a typed edge carrying its relation and                        the source/file that declared it. Bounded — `truncated` marks                        a view cut at the limit, `partial` marks sources that did not                        answer.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `limit` | нет | integer \| null | Maximum nodes and edges each (default 500). |
| `scope_handle` | нет | string \| null |  |
| `snapshot_handle` | нет | string \| null |  |
| `strict` | нет | boolean \| null | Strict reads (default) fail with `source_unavailable` when a<br>required source cannot answer; `false` accepts a partial answer. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "limit": {
      "description": "Maximum nodes and edges each (default 500).",
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
  "title": "GraphParams",
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
