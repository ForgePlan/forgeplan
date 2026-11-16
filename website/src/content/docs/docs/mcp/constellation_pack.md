---
title: constellation_pack
description: "constellation_pack — constellation-read MCP profile"
---

## Input parameters

Supply exactly one of `scope_handle` and `snapshot_handle`. Selection uses
lexical ranking and optional explicit artifact selectors; it does not infer
authority from artifact kind. Declared grounds, informing evidence, conflicts
and replacement links are followed within the admitted sources.

| Parameter | Default | Meaning |
|---|---:|---|
| `task` | required | Task text used for lexical ranking |
| `seeds` | `[]` | Artifact keys or `key@source` selectors |
| `budget` | 8000 | Estimated full-result tokens, rounded up from UTF-8 bytes divided by four; maximum 2,000,000 |
| `byte_cap` | 1,000,000 | Maximum serialized tool-result bytes; maximum 8,000,000 |
| `max_depth` | 4 | Maximum relation depth; maximum 64 |
| `max_nodes` | 256 | Maximum visited nodes; maximum 4096 |
| `max_edges` | 1024 | Maximum traversed edges; maximum 16384 |
| `max_millis` | 5000 | Maximum traversal milliseconds; maximum 60000 |
| `strict` | true | Refuse an unavailable required source |
| `require_complete` | false | Refuse incomplete sources, obligations or paginated delivery |
| `cursor` | omitted | Continue with identical task, sources and options |
| `retrieval_mode` | lexical | Only `lexical` is supported |
| `mermaid` | false | Include navigation derived from the declared graph |

The effective byte cap is the smaller of `byte_cap` and four times `budget`.
The measurement includes the serialized MCP tool-result envelope and escaping,
but not JSON-RPC request framing. The embedded pack uses compact JSON rather
than spending the budget on indentation. `receipt.measured_bytes` reports the
full result size.
The token figure is an estimate, not model tokenization.

`source_complete`, `traversal_complete`, and `delivery_complete` describe
different coverage conditions. A `next_cursor` continues delivery of the same
selected whole units; it does not resume a traversal stopped by a limit. If the
next unit cannot fit, increase both limits rather than expecting a fragment.
The cursor is not an authorization credential.

`graph.scope` is `page_incident`: nodes are the delivered units and edges are
the captured declared edges incident to them. Edge endpoints retain their
source-qualified identities, including endpoints outside the page; unresolved
targets remain visible. Union the pages by node and edge identity to reconstruct
the bounded selected graph. `graph.total_nodes` and `graph.total_edges` count
that bounded graph, and `graph.complete` says whether this page contains it
all. These fields do not imply source or traversal completeness. Mermaid
navigation uses the same page graph, including explicit unresolved target stubs.
Cursors bind the delivery layout version; restart a pack if a continuation from
an older layout is rejected.

## Examples

- [MCP setup and workspace scope](/docs/guides/mcp-setup/)
- [Constellation walkthrough and handle sequence](/docs/guides/constellation/)

## See also

- [MCP reference](/docs/mcp/)
- [Constellation reference](/docs/reference/constellation/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `constellation-read`.

Build a source-qualified lexical ContextPack with declared relation closure, whole evidence units, bounded delivery and continuation.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `budget` | no | integer \| null | Estimated full-result allowance (UTF-8 bytes divided by four, rounded up). |
| `byte_cap` | no | integer \| null | Exact cap on serialized MCP tool result bytes. |
| `cursor` | no | string \| null | Continuation from the previous response with unchanged inputs. |
| `max_depth` | no | integer \| null | Maximum relation depth (default 4, maximum 64). |
| `max_edges` | no | integer \| null | Maximum declared edges (default 1024, maximum 16384). |
| `max_millis` | no | integer \| null | Maximum traversal milliseconds (default 5000, maximum 60000). |
| `max_nodes` | no | integer \| null | Maximum visited artifacts (default 256, maximum 4096). |
| `mermaid` | no | boolean \| null | Include Mermaid navigation for the declared graph. |
| `require_complete` | no | boolean \| null | Require complete sources, traversal and delivery in one result. |
| `retrieval_mode` | no | string \| null | Retrieval mode; only `lexical` is supported. |
| `scope_handle` | no | string \| null |  |
| `seeds` | no | array \| null | Explicit source-qualified artifact selectors. |
| `snapshot_handle` | no | string \| null |  |
| `strict` | no | boolean \| null | Strict reads (default) fail with `source_unavailable` when a<br>required source cannot answer; `false` accepts a partial answer. |
| `task` | yes | string | What the agent must do — ranking terms come from this text. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "budget": {
      "description": "Estimated full-result allowance (UTF-8 bytes divided by four, rounded up).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "byte_cap": {
      "description": "Exact cap on serialized MCP tool result bytes.",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "cursor": {
      "description": "Continuation from the previous response with unchanged inputs.",
      "type": [
        "string",
        "null"
      ]
    },
    "max_depth": {
      "description": "Maximum relation depth (default 4, maximum 64).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "max_edges": {
      "description": "Maximum declared edges (default 1024, maximum 16384).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "max_millis": {
      "description": "Maximum traversal milliseconds (default 5000, maximum 60000).",
      "format": "uint64",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "max_nodes": {
      "description": "Maximum visited artifacts (default 256, maximum 4096).",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "mermaid": {
      "description": "Include Mermaid navigation for the declared graph.",
      "type": [
        "boolean",
        "null"
      ]
    },
    "require_complete": {
      "description": "Require complete sources, traversal and delivery in one result.",
      "type": [
        "boolean",
        "null"
      ]
    },
    "retrieval_mode": {
      "description": "Retrieval mode; only `lexical` is supported.",
      "type": [
        "string",
        "null"
      ]
    },
    "scope_handle": {
      "type": [
        "string",
        "null"
      ]
    },
    "seeds": {
      "description": "Explicit source-qualified artifact selectors.",
      "items": {
        "type": "string"
      },
      "type": [
        "array",
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
    },
    "task": {
      "description": "What the agent must do — ranking terms come from this text.",
      "type": "string"
    }
  },
  "required": [
    "task"
  ],
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
