---
title: forgeplan_search
description: "Smart search across artifacts: BM25 keyword + optional semantic + graph expansion. Supports filters by kind/status/depth/evidence/since and graph expansion toggle."
---

Full-text discovery over every artifact in the workspace. v0.18.0 uses production BM25 (`bm25` crate) with Russian morphology (Snowball stemmer), template noise stripping, and O(N) batch search. This is the agent's primary tool for "find anything about X" questions — much richer than `forgeplan_list` (which is metadata-only) and much cheaper than a `forgeplan_get` loop.

**Category**: Reading Artifacts

## When an agent calls this

- User asks "do we already have a decision about retries?" — agent searches before creating a new ADR.
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

_Schema source: `crates/forgeplan-mcp/src/server.rs::SearchParams`_

## Returns

A ranked array of hits. Each hit has the artifact ID, kind, status, title, a snippet (matched section), and the BM25 score. When `expand_graph: true`, linked neighbours are also included with a `via` field explaining the link type.

Example response shape:

```json
{
  "query": "authentication flow",
  "hits": [
    { "id": "PRD-042", "kind": "prd", "status": "active", "score": 12.4, "snippet": "...OAuth2 authentication flow with refresh tokens..." },
    { "id": "RFC-018", "kind": "rfc", "status": "active", "score": 9.8, "snippet": "...implements the auth flow defined in PRD-042..." }
  ]
}
```

## Example invocation

```json
{ "query": "authentication flow", "limit": 5 }
```

With typical agent context:

> Before proposing a new retry strategy, agent searches for any existing decision on the topic.

```json
{ "query": "retry backoff strategy", "kind": "adr", "limit": 10 }
```

## Typical sequence

`forgeplan_search` → agent picks top hit → `forgeplan_get` to read full body → decide whether to edit, supersede, or create new. In the Forgeplan session protocol, `search` is a mandatory pre-flight before any `new` to avoid duplication.

## CLI equivalent

- [`forgeplan search`](/docs/cli/search/) — same engine, terminal output

## See also

- [MCP overview](/docs/mcp/)
- [v0.18.0 BM25 search guide](/docs/guides/search-v2/)
- [`forgeplan_fpf_search`](/docs/mcp/forgeplan_fpf_search/) — FPF KB semantic search
- [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) — exploratory discovery
