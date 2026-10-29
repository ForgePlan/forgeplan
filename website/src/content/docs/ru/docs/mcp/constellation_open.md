---
title: constellation_open
description: "constellation_open — constellation-read MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `constellation-read`.

Open a read scope: resolve a named (or the active) constellation's                        source list once and return an opaque `scope-…` handle. Later reads                        use the handle — the source set cannot widen mid-session even if                        `constellation use` moves elsewhere.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `constellation` | нет | string \| null | Constellation name. Omit for the active one. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "constellation": {
      "description": "Constellation name. Omit for the active one.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "OpenParams",
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
