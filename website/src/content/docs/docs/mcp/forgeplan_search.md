---
title: forgeplan_search
description: "Search artifact bodies with BM25 smart search or keyword substring matching. Forgeplan 0.38.0 MCP has no semantic vectors; filters constrain smart direct matches, not expanded neighbors. Use no_expand for a filter-constrained result set."
---

Full-text discovery over every artifact in the workspace. v0.18.0 uses production BM25 (`bm25` crate) with Russian morphology (Snowball stemmer), template noise stripping, and O(N) batch search. This is the agent's primary tool for "find anything about X" questions - much richer than `forgeplan_list` (which is metadata-only) and much cheaper than a `forgeplan_get` loop.

**Category**: Reading Artifacts

## When an agent calls this

- User asks "do we already have a decision about retries?" - agent searches before creating a new ADR.
- Duplicate check before `forgeplan_new`: confirm no existing PRD already covers the topic.
- Context gathering for `forgeplan_reason`: pull 3-5 related artifacts to seed the ADI prompt.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `depth` | `string \| null` | no | Filter by depth (tactical, standard, deep, critical). Schema default: `null`. |
| `kind` | `string \| null` | no | Filter by artifact kind (e.g. "prd", "rfc"). Schema default: `null`. |
| `limit` | `integer` | no | Max results (default 20). Schema default: `20`. `format`: `uint`. `minimum`: `0`. |
| `mode` | `string \| null` | no | Search mode: "keyword", "semantic", or "smart" (default). Schema default: `null`. |
| `no_evidence` | `boolean` | no | Only include artifacts without evidence (R_eff == 0). Schema default: `false`. |
| `no_expand` | `boolean` | no | Disable 1-hop graph expansion of top results. Schema default: `false`. |
| `query` | `string` | yes | Search query (BM25 keyword + optional semantic, case-insensitive). |
| `since` | `string \| null` | no | Filter by created_at date (YYYY-MM-DD). Schema default: `null`. |
| `status` | `string \| null` | no | Filter by status (e.g. "active", "draft"). Schema default: `null`. |
| `with_evidence` | `boolean` | no | Only include artifacts with linked evidence (R_eff > 0). Schema default: `false`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is a JSON object in MCP `content[0].text` with `results`, `total`, and `_next_action` (not `hits` or `query`). Each result has `id`, `kind`, `title`, `status`, `matched_lines`, `score`, `bm25_score`, `semantic_score`, `r_eff`, `id_canonical`, and `id_display`. Optional `slug`, `predicted_number`, and `assigned_number` are omitted when absent. Graph-expanded results may include `expanded_from`, the parent artifact ID, not a relation name.

In 0.38.0, default `smart` and `semantic` both call the same BM25 smart-search path **without semantic vectors** (#546). `matched_lines` is empty on that path. Graph expansion is attempted by default; `no_expand: true` disables it. `keyword` instead uses substring matching, returns at most five line-numbered matches per result, and sets all three search scores to zero. It applies `kind` and `limit` but ignores the composite `status`, `depth`, evidence, and `since` filters (#544); `since` is still parsed before this branch. `limit: 0` becomes 20. A blank query is a plain-text tool error; an invalid `since` date is JSON-RPC invalid params.

Smart/semantic filters apply to **direct matches only**. Default graph expansion then adds neighbors from the full record set without reapplying `kind`, `status`, `depth`, evidence, or date filters. The final result can therefore include an out-of-filter neighbor. Set `no_expand: true` when the entire smart/semantic result set must satisfy the requested filters; this does not fix keyword mode's ignored composite filters.

Illustrative keyword response (not an observed result):

```json
{
  "results": [
    { "id": "PRD-042", "kind": "prd", "title": "Auth system", "status": "active", "matched_lines": ["L12: OAuth2 authentication flow with refresh tokens"], "score": 0.0, "bm25_score": 0.0, "semantic_score": 0.0, "r_eff": 0.0, "id_canonical": "prd-042", "id_display": "PRD-042" }
  ],
  "total": 1,
  "_next_action": "1 hit(s). Read top result: `forgeplan_get PRD-042`."
}
```

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "query": "authentication flow",
  "limit": 5
}
```

## Typical sequence

`forgeplan_search` → agent picks a candidate → `forgeplan_get` reads its body → agent decides whether to edit, supersede, or create. Searching before creation is a recommended agent workflow, not a duplicate-prevention gate enforced by this handler.

## CLI equivalent

- [`forgeplan search`](/docs/cli/search/) - same engine, terminal output

## See also

- [MCP overview](/docs/mcp/)
- [v0.18.0 BM25 search guide](/docs/guides/search-v2/)
- [`forgeplan_fpf_search`](/docs/mcp/forgeplan_fpf_search/) - FPF KB semantic search
- [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) - exploratory discovery
