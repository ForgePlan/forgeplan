---
title: forgeplan_release_notes
description: "forgeplan_release_notes — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Auto-generate Keep-a-Changelog–shaped release notes from artifacts that changed between two git refs. Walks `git log` over `.forgeplan/{prds, problems, evidence, rfcs, adrs, specs, epics, solutions}/`, classifies each touched artifact (PRD→Added, PROB→Fixed, EVID-on-security→Security, RFC/ADR→Changed) and emits the structured payload. Quality gate (default): only artifacts with status==active or r_eff_score > 0 are included. Pass `draft: true` to waive the gate. `since` defaults to the latest git tag, `until` defaults to HEAD. Example: `{since: "v0.30.0", draft: false}`.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `draft` | no | boolean | Disable the quality gate (include active artifacts without<br>evidence). Default: false. |
| `since` | no | string \| null | Git ref to start from (e.g. `v0.30.0`). Default: latest tag. |
| `until` | no | string \| null | Git ref to end at. Default: HEAD. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

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

## Examples

- [MCP setup and workspace scope](/docs/guides/mcp-setup/)
- [Constellation walkthrough and handle sequence](/docs/guides/constellation/)

## See also

- [MCP reference](/docs/mcp/)
- [Constellation reference](/docs/reference/constellation/)
