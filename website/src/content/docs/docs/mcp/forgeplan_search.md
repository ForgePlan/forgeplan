---
title: forgeplan_search
description: "Smart search across artifacts: BM25 keyword + optional semantic + graph expansion. Supports filters by kind/status/depth/evidence/since and graph expansion toggle."
---

Discovers workspace artifacts without fetching every full body. The v0.18.0 BM25/morphology description is historical background, not a claim that every current mode follows that pipeline. In the selected local MCP handler, keyword mode uses case-insensitive substring matching; other mode strings use the shared smart-search path.

**Category**: Reading Artifacts

## When an agent calls this

- User asks "do we already have a decision about retries?" - agent searches before creating a new ADR.
- Duplicate check before `forgeplan_new`: confirm no existing PRD already covers the topic.
- Context gathering for `forgeplan_reason`: pull 3-5 related artifacts to seed the ADI prompt.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `query` | `string` | yes | Search query (BM25 keyword + optional semantic, case-insensitive). |
| `kind` | `string` | no | Filter by artifact kind (e.g. `prd`, `rfc`). |
| `status` | `string` | no | Filter by status (e.g. `active`, `draft`). |
| `depth` | `string` | no | Filter by depth (`tactical`, `standard`, `deep`, `critical`). |
| `with_evidence` | `bool` | no (default: `false`) | Only include artifacts with linked evidence (R_eff > 0). |
| `no_evidence` | `bool` | no (default: `false`) | Only include artifacts without evidence (R_eff == 0). |
| `since` | `string` | no | Filter by `created_at` date (YYYY-MM-DD). |
| `no_expand` | `bool` | no (default: `false`) | Disable 1-hop graph expansion of top results. |
| `limit` | `integer` | no (default: `20`) | Max results to return. |
| `mode` | `string` | no | Search mode: `keyword`, `semantic`, or `smart` (default). |
| `workspace` | `string?` | no | Absolute or `~/`-expanded path. Overrides `FORGEPLAN_WORKSPACE`, then server default, then CWD parent discovery. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::SearchParams`_

## Returns

The MCP `CallToolResult` carries JSON text in `content[]`. Its parsed payload
is a `SearchResponse` object, not a bare array:

- `results`: ranked entries with `id`, `kind`, `title`, `status`,
  `matched_lines`, `score`, `bm25_score`, `semantic_score`, `r_eff`,
  `id_canonical`, `id_display`; optional `slug`, `predicted_number`,
  `assigned_number`, and `expanded_from`.
- `total`: result count.
- `coverage`: booleans `semantic`, `lexical`, `graph` reporting channel state. In the smart path, `graph` is set from `!no_expand`, even if graph retrieval failed; it is not proof that graph neighbours contributed to this response.

Use input `no_expand: true` to disable expansion; there is no input
`expand_graph`. Expanded entries identify the parent through `expanded_from`,
not `via`. Matched text uses `matched_lines`, not `snippet`.

In local `0.39.0+constellation.s1.1`, queries do not download missing model
weights. Unlike the CLI's forced-semantic contract, this MCP handler does
not have a separate forced-semantic branch: `semantic` and even unknown
mode strings enter the shared smart path. Inspect the returned coverage.

Keyword mode honors `kind` and the result limit but does not apply the
constructed `status`, `depth`, evidence or `since` filters; it also skips
graph expansion. Use the smart path when those filters matter, and inspect
results rather than assuming the schema proves enforcement. Smart results
currently have empty `matched_lines`; fetch their bodies with `forgeplan_get`.

The default limit is 20 (`0` is also mapped to 20); the separate pinned
Constellation search defaults to 10. See [Constellation setup](/docs/guides/constellation/)
and [contracts](/docs/reference/constellation/).

## Example invocation

```json
{ "query": "authentication flow", "limit": 5, "workspace": "/path/to/project" }
```

With typical agent context:

> Before proposing a new retry strategy, agent searches for any existing decision on the topic.

```json
{ "query": "retry backoff strategy", "kind": "adr", "limit": 10 }
```

## Typical sequence

`forgeplan_search` → agent picks top hit → `forgeplan_get` to read full body → decide whether to edit, supersede, or create new. In the Forgeplan session protocol, `search` is a mandatory pre-flight before any `new` to avoid duplication.

## CLI equivalent

- [`forgeplan search`](/docs/cli/search/) - same engine, terminal output

## See also

- [MCP overview](/docs/mcp/)
- [v0.18.0 BM25 search guide](/docs/guides/search-v2/)
- [`forgeplan_fpf_search`](/docs/mcp/forgeplan_fpf_search/) - FPF KB semantic search
- [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) - exploratory discovery

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Smart search across artifacts: BM25 keyword + optional semantic + graph expansion. Supports filters by kind/status/depth/evidence/since and graph expansion toggle.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `depth` | no | string \| null | Filter by depth (tactical, standard, deep, critical). |
| `kind` | no | string \| null | Filter by artifact kind (e.g. "prd", "rfc"). |
| `limit` | no | integer | Max results (default 20). |
| `mode` | no | string \| null | Search mode: "keyword", "semantic", or "smart" (default). |
| `no_evidence` | no | boolean | Only include artifacts without evidence (R_eff == 0). |
| `no_expand` | no | boolean | Disable 1-hop graph expansion of top results. |
| `query` | yes | string | Search query (BM25 keyword + optional semantic, case-insensitive). |
| `since` | no | string \| null | Filter by created_at date (YYYY-MM-DD). |
| `status` | no | string \| null | Filter by status (e.g. "active", "draft"). |
| `with_evidence` | no | boolean | Only include artifacts with linked evidence (R_eff &gt; 0). |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "depth": {
      "default": null,
      "description": "Filter by depth (tactical, standard, deep, critical).",
      "type": [
        "string",
        "null"
      ]
    },
    "kind": {
      "default": null,
      "description": "Filter by artifact kind (e.g. \"prd\", \"rfc\").",
      "type": [
        "string",
        "null"
      ]
    },
    "limit": {
      "default": 20,
      "description": "Max results (default 20).",
      "format": "uint",
      "minimum": 0,
      "type": "integer"
    },
    "mode": {
      "default": null,
      "description": "Search mode: \"keyword\", \"semantic\", or \"smart\" (default).",
      "type": [
        "string",
        "null"
      ]
    },
    "no_evidence": {
      "default": false,
      "description": "Only include artifacts without evidence (R_eff == 0).",
      "type": "boolean"
    },
    "no_expand": {
      "default": false,
      "description": "Disable 1-hop graph expansion of top results.",
      "type": "boolean"
    },
    "query": {
      "description": "Search query (BM25 keyword + optional semantic, case-insensitive).",
      "type": "string"
    },
    "since": {
      "default": null,
      "description": "Filter by created_at date (YYYY-MM-DD).",
      "type": [
        "string",
        "null"
      ]
    },
    "status": {
      "default": null,
      "description": "Filter by status (e.g. \"active\", \"draft\").",
      "type": [
        "string",
        "null"
      ]
    },
    "with_evidence": {
      "default": false,
      "description": "Only include artifacts with linked evidence (R_eff > 0).",
      "type": "boolean"
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
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
