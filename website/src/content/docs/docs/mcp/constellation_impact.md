---
title: constellation_impact
description: "constellation_impact — constellation-read MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `constellation-read`.

Explainable impact: who depends on the artifact, by which                        relation path, and where the bounded walk stopped. `based_on`                        and friends propagate transitively; `references`/`informs`                        only mark the direct neighbor.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `depth` | no | integer \| null | Maximum structural hops (default 4). |
| `id` | yes | string | The changed artifact's canonical key or display id. |
| `max_nodes` | no | integer \| null | Maximum impacted nodes (default 256). |
| `scope_handle` | no | string \| null |  |
| `snapshot_handle` | no | string \| null |  |
| `strict` | no | boolean \| null | Strict reads (default) fail with `source_unavailable` when a<br>required source cannot answer; `false` accepts a partial answer. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "depth": {
      "description": "Maximum structural hops (default 4).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "id": {
      "description": "The changed artifact's canonical key or display id.",
      "type": "string"
    },
    "max_nodes": {
      "description": "Maximum impacted nodes (default 256).",
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
    }
  },
  "required": [
    "id"
  ],
  "title": "ImpactParams",
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
