---
title: forgeplan_playbook_show
description: "forgeplan_playbook_show — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Show full details of a single playbook (resolved by name or path). Returns the parsed Playbook struct + source path. PRD-065 AC-1.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `target` | yes | string | Playbook name (matches discovery results) or path to a YAML file. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Params for `forgeplan_playbook_show` — accepts either a kebab-case\nplaybook name or a path to a `.yaml` file.",
  "properties": {
    "target": {
      "description": "Playbook name (matches discovery results) or path to a YAML file.",
      "type": "string"
    }
  },
  "required": [
    "target"
  ],
  "title": "PlaybookShowParams",
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
