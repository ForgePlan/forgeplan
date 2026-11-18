---
title: forgeplan_unlink
description: "forgeplan_unlink — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Remove a typed relation between two artifacts. Mirror of `forgeplan_link` for undoing or fixing mis-typed edges. Issue #286.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `relation` | нет | RelationKind | Relationship type to remove (default: informs) |
| `source` | да | string | Source artifact ID |
| `target` | да | string | Target artifact ID |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "RelationKind": {
      "enum": [
        "informs",
        "based_on",
        "supersedes",
        "contradicts",
        "refines"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "relation": {
      "$ref": "#/$defs/RelationKind",
      "description": "Relationship type to remove (default: informs)"
    },
    "source": {
      "description": "Source artifact ID",
      "type": "string"
    },
    "target": {
      "description": "Target artifact ID",
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
    "source",
    "target"
  ],
  "title": "UnlinkParams",
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
