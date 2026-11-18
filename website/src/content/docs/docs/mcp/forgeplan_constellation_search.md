---
title: forgeplan_constellation_search
description: "forgeplan_constellation_search — full MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

In the selected local implementation, this full-profile tool first attempts
cached pinned semantic search; on unavailability or a semantic failure it
can answer lexically. Inspect `mode`, `semantic_available`,
`semantic_unavailable` or `semantic_error`, and `eligible_rows` where returned.
The captured catalog's lexical-only/later-ANN description below is stale
relative to that handler. A query never downloads missing model weights.
The default limit is 10, clamped to 1..=100. This differs from
`forgeplan_search` and the read-profile handle-based `constellation_search`.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Lexical search over the pinned Constellation artifacts table —                        case-insensitive substring match on canonical_key, title and body                        at the *published* generation (a pinned reader, never latest).                        Each hit reports its stored `semantic_state` honestly; ANN/vector                        search is a later stage and is not claimed here.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `limit` | no | integer \| null | Max hits (default 10, hard cap 100) |
| `query` | yes | string | Substring to match (case-insensitive) against canonical_key, title, body |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call — same routing as<br>every other read tool (PRD-078 / ADR-015). |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "limit": {
      "default": null,
      "description": "Max hits (default 10, hard cap 100)",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "query": {
      "description": "Substring to match (case-insensitive) against canonical_key, title, body",
      "type": "string"
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call — same routing as\nevery other read tool (PRD-078 / ADR-015).",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "query"
  ],
  "title": "ConstellationSearchParams",
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
