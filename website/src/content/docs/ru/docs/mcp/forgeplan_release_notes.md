---
title: forgeplan_release_notes
description: "forgeplan_release_notes — full MCP profile"
---

## Входные параметры

Таблица и JSON Schema ниже взяты из живого сервера. Они описывают контракт вызова, а не подтверждают доступность всех операций на любой раскладке или без требуемого плагина.

<!-- forgeplan-runtime:start -->
## Снимок tools/list

Источник: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, профиль `full`.

Auto-generate Keep-a-Changelog–shaped release notes from artifacts that changed between two git refs. Walks `git log` over `.forgeplan/{prds, problems, evidence, rfcs, adrs, specs, epics, solutions}/`, classifies each touched artifact (PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed) and emits the structured payload. Quality gate (default): only artifacts with status==active or r_eff_score > 0 are included. Pass `draft: true` to waive the gate. `since` defaults to the latest git tag, `until` defaults to HEAD. Example: `{since: "v0.30.0", draft: false}`.

### Входные параметры

| Имя | Обязателен | Тип | Описание |
|---|---|---|---|
| `draft` | нет | boolean | Disable the quality gate (include active artifacts without<br>evidence). Default: false. |
| `since` | нет | string \| null | Git ref to start from (e.g. `v0.30.0`). Default: latest tag. |
| `until` | нет | string \| null | Git ref to end at. Default: HEAD. |
| `workspace` | нет | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "draft": {
      "default": false,
      "description": "Disable the quality gate (include active artifacts without\nevidence). Default: false.",
      "type": "boolean"
    },
    "since": {
      "description": "Git ref to start from (e.g. `v0.30.0`). Default: latest tag.",
      "type": [
        "string",
        "null"
      ]
    },
    "until": {
      "description": "Git ref to end at. Default: HEAD.",
      "type": [
        "string",
        "null"
      ]
    },
    "workspace": {
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "ReleaseNotesParams",
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
