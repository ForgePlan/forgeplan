---
title: constellation_pack
description: "constellation_pack — constellation-read MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

Although both handle keys are optional in the schema, a usable call needs
a live `scope_handle` or `snapshot_handle`. The selected implementation's
estimator is `ceil(UTF-8 bytes / 4)`, despite the catalog's “chars/4”
shorthand; it is not model tokenization or a hard bound on the serialized
response. The content-unit budget excludes envelope overhead.
Live-scope success wraps the result in `pack` with `payload_bytes`;
snapshot success returns the pack directly.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `constellation-read`.

Build a budgeted ContextPack for a task over the scope's live                        sources or a snapshot's pinned revisions: lexically ranked                        evidence units with roles, named omissions and a spend receipt.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `budget` | no | integer \| null | Token budget (estimated tokens, chars/4; default 8000). |
| `scope_handle` | no | string \| null |  |
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

## Examples

- [MCP setup and workspace scope](/docs/guides/mcp-setup/)
- [Constellation walkthrough and handle sequence](/docs/guides/constellation/)

## See also

- [MCP reference](/docs/mcp/)
- [Constellation reference](/docs/reference/constellation/)
