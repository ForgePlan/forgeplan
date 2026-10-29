---
title: forgeplan_unlink
description: "forgeplan_unlink — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Remove a typed relation between two artifacts. Mirror of `forgeplan_link` for undoing or fixing mis-typed edges. Issue #286.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `relation` | no | RelationKind | Relationship type to remove (default: informs) |
| `source` | yes | string | Source artifact ID |
| `target` | yes | string | Target artifact ID |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

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

## Examples

- [MCP setup and workspace scope](/docs/guides/mcp-setup/)
- [Constellation walkthrough and handle sequence](/docs/guides/constellation/)

## See also

- [MCP reference](/docs/mcp/)
- [Constellation reference](/docs/reference/constellation/)
