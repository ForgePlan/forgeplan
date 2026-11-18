---
title: constellation_graph
description: "constellation_graph — constellation-read MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

Although neither handle is required by the schema, a useful call needs a live `scope_handle` from `constellation_open` or a `snapshot_handle` from `constellation_snapshot` in the same server process. With neither, the handler returns `unknown_handle`. Use one handle kind per call. `limit` bounds nodes and edges separately; it is not a completeness guarantee.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `constellation-read`.

The relation graph of the scope's live sources or a snapshot's                        pinned revisions: every artifact as a source-qualified node,                        every declared link as a typed edge carrying its relation and                        the source/file that declared it. Bounded — `truncated` marks                        a view cut at the limit, `partial` marks sources that did not                        answer.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `limit` | no | integer \| null | Maximum nodes and edges each (default 500). |
| `scope_handle` | no | string \| null |  |
| `snapshot_handle` | no | string \| null |  |
| `strict` | no | boolean \| null | Strict reads (default) fail with `source_unavailable` when a<br>required source cannot answer; `false` accepts a partial answer. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "limit": {
      "description": "Maximum nodes and edges each (default 500).",
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
  "title": "GraphParams",
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
