---
title: forgeplan_constellation_get
description: "forgeplan_constellation_get — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Read ONE artifact at the pinned Constellation generation —                        exact match on `canonical_key` or the human id (`SPEC-001`).                        Returns the stored row verbatim: body, digests, semantic_state.                        A miss is `found: false`, never a substituted row.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `key` | yes | string | Canonical key or human id (`SPEC-001`) — exact, case-insensitive |
| `workspace` | no | string \| null | Optional explicit workspace directory — same routing as other reads |

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

## Examples

- [MCP setup and workspace scope](/docs/guides/mcp-setup/)
- [Constellation walkthrough and handle sequence](/docs/guides/constellation/)

## See also

- [MCP reference](/docs/mcp/)
- [Constellation reference](/docs/reference/constellation/)
