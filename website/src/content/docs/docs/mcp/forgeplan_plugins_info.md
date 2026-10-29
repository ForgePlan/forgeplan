---
title: forgeplan_plugins_info
description: "forgeplan_plugins_info — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Show details for a single plugin from the extended registry. Returns the static PluginInfo plus, if installed, the InstalledPlugin runtime record (detected path + version). PRD-067.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `name` | yes | string | Plugin name (kebab-case) as registered in the extended registry. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Params for `forgeplan_plugins_info`.",
  "properties": {
    "name": {
      "description": "Plugin name (kebab-case) as registered in the extended registry.",
      "type": "string"
    }
  },
  "required": [
    "name"
  ],
  "title": "PluginsInfoParams",
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
