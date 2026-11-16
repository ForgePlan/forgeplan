---
title: constellation_pack
description: "constellation_pack — constellation-read MCP profile"
---

## Входные параметры

Укажите ровно один действующий `scope_handle` или `snapshot_handle`.
Лексическое ранжирование и явные `seeds` выбирают начальные артефакты.
Обход сохраняет объявленные основания, свидетельства, противоречия и замены
в пределах допущенных источников; вид артефакта сам по себе не придаёт ему
авторитета.

`budget` ограничивает оценочный размер полного ответа, включая оболочку
MCP и экранирование: UTF-8 байты / 4 с округлением вверх. Это не токенизация
модели. Точный предел байтов равен меньшему из `byte_cap` и четырёх `budget`;
JSON-RPC framing в него не входит. Вложенный пакет использует компактный JSON.
`receipt.measured_bytes` показывает фактический размер результата.

`source_complete`, `traversal_complete` и `delivery_complete` описывают
разные условия. `next_cursor` продолжает выдачу тех же целых единиц, а не
обход, остановленный пределом. Параметры и версии должны остаться прежними.
Если следующая единица не помещается, увеличьте оба предела и начните заново.
`require_complete: true` отказывает при неполноте источников, обхода или выдаче
в несколько страниц. Курсор не даёт дополнительных прав на источники.

При `graph.scope: "page_incident"` узлы соответствуют выданным единицам,
а связи — объявленным рёбрам, касающимся этих узлов. Другой конец связи
сохраняет квалифицированный источник, даже если не выдан на этой странице.
Неразрешённые цели остаются видимыми, в том числе в Mermaid.
Объединение страниц по идентичности узлов и рёбер восстанавливает выбранный
ограниченный граф. `graph.total_nodes` и `graph.total_edges` считают именно
его, не весь корпус. `graph.complete` показывает, содержит ли страница весь
этот граф; поле не означает полноту источников или обхода.
Курсоры привязаны к версии раскладки выдачи. После отказа старого курсора
начните новый пакет.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `constellation-read`.

Build a source-qualified lexical ContextPack with declared relation closure, whole evidence units, bounded delivery and continuation.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `budget` | no | integer \| null | Estimated full-result allowance (UTF-8 bytes divided by four, rounded up). |
| `byte_cap` | no | integer \| null | Exact cap on serialized MCP tool result bytes. |
| `cursor` | no | string \| null | Continuation from the previous response with unchanged inputs. |
| `max_depth` | no | integer \| null | Maximum relation depth (default 4, maximum 64). |
| `max_edges` | no | integer \| null | Maximum declared edges (default 1024, maximum 16384). |
| `max_millis` | no | integer \| null | Maximum traversal milliseconds (default 5000, maximum 60000). |
| `max_nodes` | no | integer \| null | Maximum visited artifacts (default 256, maximum 4096). |
| `mermaid` | no | boolean \| null | Include Mermaid navigation for the declared graph. |
| `require_complete` | no | boolean \| null | Require complete sources, traversal and delivery in one result. |
| `retrieval_mode` | no | string \| null | Retrieval mode; only `lexical` is supported. |
| `scope_handle` | no | string \| null |  |
| `seeds` | no | array \| null | Explicit source-qualified artifact selectors. |
| `snapshot_handle` | no | string \| null |  |
| `strict` | no | boolean \| null | Strict reads (default) fail with `source_unavailable` when a<br>required source cannot answer; `false` accepts a partial answer. |
| `task` | yes | string | What the agent must do — ranking terms come from this text. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "budget": {
      "description": "Estimated full-result allowance (UTF-8 bytes divided by four, rounded up).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "byte_cap": {
      "description": "Exact cap on serialized MCP tool result bytes.",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "cursor": {
      "description": "Continuation from the previous response with unchanged inputs.",
      "type": [
        "string",
        "null"
      ]
    },
    "max_depth": {
      "description": "Maximum relation depth (default 4, maximum 64).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "max_edges": {
      "description": "Maximum declared edges (default 1024, maximum 16384).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "max_millis": {
      "description": "Maximum traversal milliseconds (default 5000, maximum 60000).",
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "max_nodes": {
      "description": "Maximum visited artifacts (default 256, maximum 4096).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "mermaid": {
      "description": "Include Mermaid navigation for the declared graph.",
      "type": [
        "boolean",
        "null"
      ]
    },
    "require_complete": {
      "description": "Require complete sources, traversal and delivery in one result.",
      "type": [
        "boolean",
        "null"
      ]
    },
    "retrieval_mode": {
      "description": "Retrieval mode; only `lexical` is supported.",
      "type": [
        "string",
        "null"
      ]
    },
    "scope_handle": {
      "type": [
        "string",
        "null"
      ]
    },
    "seeds": {
      "description": "Explicit source-qualified artifact selectors.",
      "items": {
        "type": "string"
      },
      "type": [
        "array",
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
