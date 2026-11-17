---
title: forgeplan_fpf_search
description: "Search FPF (First Principles Framework) knowledge base. Default is keyword search. Pass `semantic: true` for vector similarity search via BGE-M3 embeddings (requires the `semantic-search` build feature). When `semantic: true` but the feature is not compiled in, the query gracefully falls back to keyword search and the response includes a `warning` field. Note: the first invocation with `semantic: true` may take several minutes if the BGE-M3 model needs to be downloaded (~2.1 GB). Params: query (required, 1..=8192 chars), limit (default 5, max 50), semantic (default false)."
---

Searches the **FPF knowledge base loaded in the workspace**. Keyword search is the default; `semantic: true` requests BGE-M3 vector search when the build supports it. The model, its cache and the ingested corpus are prerequisites for that path; feature, model or vector-search failures can fall back to keyword search with a `warning`.

**Category**: FPF Knowledge Base

## When an agent calls it

- **Looking up a principle** - "what does FPF say about trust calculus?" → agents often hit B.3.
- **Grounding ADI reasoning** - before generating hypotheses, fetch relevant FPF context.
- **Decision support** - `fpf_search "explore exploit"` surfaces the exploration/exploitation balance rules.
- **Onboarding** - help a new agent discover FPF vocabulary quickly.

Run CLI `forgeplan fpf ingest` if the server reports that the knowledge base is not loaded. Do not assume the corpus has a fixed number of sections or that this endpoint shares every ranking detail with artifact search.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `limit` | `integer \| null` | no | Max results (default 5, max 50) Schema default: `null`. `format`: `uint`. `minimum`: `0`. |
| `query` | `string` | yes | Search query - keyword or semantic depending on `semantic` flag |
| `semantic` | `boolean \| null` | no | Use semantic (vector) search instead of keyword. Requires `semantic-search` feature at build time. When the feature is not compiled in, the query gracefully falls back to keyword search and the response includes a `warning` field explaining why. Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP text content with `query`, `semantic`, `count`, `results`, `warning` (string or `null`) and `_next_action`. Each hit has `id`, `section_id`, `title`, `snippet` and `line_count`; there is no numeric `score` or `path` field.

Use **`section_id`** as the next `forgeplan_fpf_section` input `id`, not the hit's internal `id`. `semantic` records the requested mode, so inspect `warning` to detect a keyword fallback rather than inferring that vector search succeeded from `semantic: true`. This response has no `mode` or `total` field.

The handler rejects blank queries and queries over 8192 bytes with `isError` plain text. Default limit is 5, capped at 50. Snippets join the first three body lines and truncate to 200 characters; they are not necessarily query-centered. Keyword matching uses title/body terms with OR semantics and title weighting, not artifact BM25 ranking. A keyword-store failure is logged and can produce an empty successful result, so zero hits alone does not prove no matching text exists.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "query": "trust calculus",
  "limit": 5
}
```

## Typical sequence

1. `forgeplan_fpf_search` with the concept you need.
2. `forgeplan_fpf_section` with the top `section_id` as its input `id` - read full body.
3. Review the text in the agent's context, or use CLI `forgeplan reason --fpf` when that optional grounding is required.

## CLI equivalent

```bash
forgeplan fpf search "trust calculus"
forgeplan fpf search "uncertainty" --semantic
```

## See also

- [`forgeplan_fpf_section`](/docs/mcp/forgeplan_fpf_section/) - fetch a section by ID.
- [`forgeplan_search`](/docs/mcp/forgeplan_search/) - search workspace artifacts (not FPF KB).
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) - ADI reasoning; optional `--fpf` belongs to the CLI, not MCP input.
- [Methodology guide](/docs/methodology/overview/)
