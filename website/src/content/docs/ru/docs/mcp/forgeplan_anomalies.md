---
title: forgeplan_anomalies
description: "forgeplan_anomalies — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Issue #289 — detect pipeline anomalies (stuck drafts, orphan links, mis-typed based_on, missing MUST sections, expired evidence, cycles, duplicates, etc.). Returns structured list with severity + tier + suggested_resolution so orchestrators (`/forge-cycle`, `/forge-cleanup`) can dispatch auto/adi/user tiers without re-classifying.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `kind` | нет | string \| null | Issue #289: filter by anomaly kind. Accepts the same snake_case<br>names emitted in the response (`stuck_draft`, `orphan_link`, …).<br>When `None`, all detected anomalies are returned. |
| `severity` | нет | string \| null | Filter by severity (`low` / `medium` / `high`). Exact match. |
| `since` | нет | string \| null | ISO 8601 cutoff — only return anomalies observed at or after this<br>timestamp. Useful for diff-style polling. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "kind": {
      "default": null,
      "description": "Issue #289: filter by anomaly kind. Accepts the same snake_case\nnames emitted in the response (`stuck_draft`, `orphan_link`, …).\nWhen `None`, all detected anomalies are returned.",
      "type": [
        "string",
        "null"
      ]
    },
    "severity": {
      "default": null,
      "description": "Filter by severity (`low` / `medium` / `high`). Exact match.",
      "type": [
        "string",
        "null"
      ]
    },
    "since": {
      "default": null,
      "description": "ISO 8601 cutoff — only return anomalies observed at or after this\ntimestamp. Useful for diff-style polling.",
      "type": [
        "string",
        "null"
      ]
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
  "title": "AnomaliesParams",
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
