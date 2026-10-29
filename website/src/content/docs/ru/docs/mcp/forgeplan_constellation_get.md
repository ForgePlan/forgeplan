---
title: forgeplan_constellation_get
description: "forgeplan_constellation_get — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Read ONE artifact at the pinned Constellation generation —                        exact match on `canonical_key` or the human id (`SPEC-001`).                        Returns the stored row verbatim: body, digests, semantic_state.                        A miss is `found: false`, never a substituted row.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `key` | да | string | Canonical key or human id (`SPEC-001`) — exact, case-insensitive |
| `workspace` | нет | string \| null | Optional explicit workspace directory — same routing as other reads |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "key": {
      "description": "Canonical key or human id (`SPEC-001`) — exact, case-insensitive",
      "type": "string"
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory — same routing as other reads",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "key"
  ],
  "title": "ConstellationGetParams",
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
