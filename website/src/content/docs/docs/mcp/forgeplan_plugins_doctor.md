---
title: forgeplan_plugins_doctor
description: "forgeplan_plugins_doctor — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Health-check across the full plugin registry: separates ok / outdated / missing entries and surfaces install hints for the missing ones. PRD-067 AC-2.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| — | — | — | No parameters. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Empty params for tools that take no arguments. We declare it explicitly\nso the JSON-Schema export advertises a (correct) empty object instead of\nrmcp falling back to `Parameters<()>` quirks.",
  "title": "EmptyParams",
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
