---
title: forgeplan_playbook_validate
description: "forgeplan_playbook_validate — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Validate a playbook YAML file (parse + structural checks: cycles, unknown step refs, mapping/produces_at consistency). Returns `passed` flag + list of errors. PRD-065 AC-2.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `file` | yes | string | Path to the playbook YAML file to validate. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Params for `forgeplan_playbook_validate`.",
  "properties": {
    "file": {
      "description": "Path to the playbook YAML file to validate.",
      "type": "string"
    }
  },
  "required": [
    "file"
  ],
  "title": "PlaybookValidateParams",
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
