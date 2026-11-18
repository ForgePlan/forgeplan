---
title: constellation_open
description: "constellation_open — constellation-read MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `constellation-read`.

Open a read scope: resolve a named (or the active) constellation's                        source list once and return an opaque `scope-…` handle. Later reads                        use the handle — the source set cannot widen mid-session even if                        `constellation use` moves elsewhere.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `constellation` | no | string \| null | Constellation name. Omit for the active one. |

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

## Examples

- [MCP setup and workspace scope](/docs/guides/mcp-setup/)
- [Constellation walkthrough and handle sequence](/docs/guides/constellation/)

## See also

- [MCP reference](/docs/mcp/)
- [Constellation reference](/docs/reference/constellation/)
