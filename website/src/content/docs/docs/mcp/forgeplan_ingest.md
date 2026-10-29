---
title: forgeplan_ingest
description: "forgeplan_ingest — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

Implementation limit in this snapshot: the MCP handler validates confined paths and size bounds, echoes the flags, and returns `wave3_status: "planned"` with an empty `drafts` array. Contrary to the emitted description below, it does not run the ingest engine or produce draft artifacts. Setting `dry_run: false` still does not write artifacts through MCP; use the CLI for execution. `dry_run` and `update` may be omitted but do not accept JSON `null`.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Plan an ingest run (mapping YAML × source file). Wave 3: dry-run-only — returns drafts without writing artifacts. Set `dry_run: false` to indicate intent to write, but the MCP surface still defers actual writes to the `forgeplan ingest` CLI (Wave 4 will wire artifact::Store). PRD-066 AC-1.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `dry_run` | no | boolean | Plan-only — engine runs but no artifacts written. Wave 3 default. |
| `mapping` | yes | string | Path to mapping YAML file (SPEC-004 schema). |
| `source` | yes | string | Path to the source file (or directory) the mapping consumes. |
| `update` | no | boolean | Allow updating existing artifacts (idempotent re-ingest). |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Params for `forgeplan_ingest`.",
  "properties": {
    "dry_run": {
      "default": true,
      "description": "Plan-only — engine runs but no artifacts written. Wave 3 default.",
      "type": "boolean"
    },
    "mapping": {
      "description": "Path to mapping YAML file (SPEC-004 schema).",
      "type": "string"
    },
    "source": {
      "description": "Path to the source file (or directory) the mapping consumes.",
      "type": "string"
    },
    "update": {
      "default": false,
      "description": "Allow updating existing artifacts (idempotent re-ingest).",
      "type": "boolean"
    }
  },
  "required": [
    "mapping",
    "source"
  ],
  "title": "IngestParams",
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
