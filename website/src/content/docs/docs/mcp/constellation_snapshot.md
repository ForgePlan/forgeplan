---
title: constellation_snapshot
description: "constellation_snapshot — constellation-read MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

Pass a live `scope-…` handle from `constellation_open` in the same server process. The shared schema's handle description also mentions `snap-…`, but this handler rejects a snapshot handle with `wrong_handle_kind`; it does not snapshot a snapshot. Non-git sources are reported as skipped rather than read live under the snapshot.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `constellation-read`.

Pin the scope's git sources at their current HEAD and return a                        `snap-…` handle. Non-git sources are recorded in the lockfile as                        skipped — a snapshot read reports them, it does not silently                        read them live.

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
