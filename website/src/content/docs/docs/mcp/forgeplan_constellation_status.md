---
title: forgeplan_constellation_status
description: "forgeplan_constellation_status — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Constellation index status — the published generation, its coverage,                        and the pinned table versions it serves from. This is the corrected                        0A/0B index; `present: false` means `forgeplan index` has not run on                        this workspace yet.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| — | — | — | No parameters. |

<details>
<summary>JSON Schema</summary>

```json
{
  "properties": {},
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
