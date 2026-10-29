---
title: constellation_sources
description: "constellation_sources — constellation-read MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

For this operation, `handle` must be a live scope handle. The shared `HandleParams` description below also mentions snapshots because other tools accept them, but `constellation_sources` returns `wrong_handle_kind` for a snapshot and `unknown_handle` for an absent handle. A listed source is not proof of embedding readiness.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `constellation-read`.

List the sources bound to a scope handle.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `handle` | yes | string | A `scope-…` or `snap-…` handle returned by `open`/`snapshot`. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "handle": {
      "description": "A `scope-…` or `snap-…` handle returned by `open`/`snapshot`.",
      "type": "string"
    }
  },
  "required": [
    "handle"
  ],
  "title": "HandleParams",
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
