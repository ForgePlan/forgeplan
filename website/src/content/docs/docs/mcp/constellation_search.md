---
title: constellation_search
description: "constellation_search — constellation-read MCP profile"
---

## Input parameters

The table and JSON Schema below come from the live server. They describe the call contract, not availability on every layout or without a required plugin.

Use a handle from this running server process: a usable `scope_handle` or `snapshot_handle` is needed even though the shared schema marks both optional. Live scope reads expire when captured source bytes change; reopen rather than reusing stale scope identity. Snapshot reads report skipped/unavailable sources instead of reading them live.

This is lexical search, not semantic search. A source appearing here does not prove that its rows are embedded or eligible in a semantic index. The captured interface belongs to the selected unreleased `0.39.0+constellation.s1.1` build, not the unchanged public/global `0.38.0`.

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `constellation-read`.

Lexical search across the scope's live sources or a snapshot's                        pinned revisions. Each hit names its source and the version it                        was read at.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `limit` | no | integer \| null | Maximum hits (default 10). |
| `query` | yes | string |  |
| `scope_handle` | no | string \| null |  |
| `snapshot_handle` | no | string \| null |  |
| `strict` | no | boolean \| null | Strict reads (default) fail with `source_unavailable` when a<br>required source cannot answer; `false` accepts a partial answer.<br>Ignored for snapshot reads — the lockfile's source set is frozen. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "limit": {
      "description": "Maximum hits (default 10).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "query": {
      "type": "string"
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
      "description": "Strict reads (default) fail with `source_unavailable` when a\nrequired source cannot answer; `false` accepts a partial answer.\nIgnored for snapshot reads — the lockfile's source set is frozen.",
      "type": [
        "boolean",
        "null"
      ]
    }
  },
  "required": [
    "query"
  ],
  "title": "SearchParams",
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
