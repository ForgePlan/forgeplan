---
title: forgeplan_fpf_search
description: "Search FPF (First Principles Framework) knowledge base. Default is keyword search. Pass `semantic: true` for vector similarity search via BGE-M3 embeddings (requires the `semantic-search` build feature). When `semantic: true` but the feature is not compiled in, the query gracefully falls back to keyword search and the response includes a `warning` field. Note: the first invocation with `semantic: true` may take 10–30 seconds if the BGE-M3 model needs to be downloaded (~150MB). Params: query (required, 1..=8192 chars), limit (default 5, max 50), semantic (default false)."
---

Search the **FPF (First Principles Framework) knowledge base** — 204 structured sections covering reasoning, trust calculus (B.3), ADI cycle (B.5), bounded contexts, and more. The default is BM25 keyword search with Russian morphology; passing `semantic: true` switches to BGE-M3 vector search when the `semantic-search` build feature is compiled in, with graceful fallback otherwise.

**Category**: FPF Knowledge Base

## When an agent calls it

- **Looking up a principle** — "what does FPF say about trust calculus?" → agents often hit B.3.
- **Grounding ADI reasoning** — before generating hypotheses, fetch relevant FPF context.
- **Decision support** — `fpf_search "explore exploit"` surfaces the exploration/exploitation balance rules.
- **Onboarding** — help a new agent discover FPF vocabulary quickly.

The pipeline is identical to regular `forgeplan_search`: BM25 tokenizer with template noise stripping + Russian Snowball stemmer, optional vector rerank with BGE-M3.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `query` | `string` | yes | Search query (1-8192 chars, trimmed non-empty). |
| `limit` | `integer` | no (default: `5`, max: `50`) | Max results to return. |
| `semantic` | `bool` | no (default: `false`) | Use vector search via BGE-M3. Falls back to keyword if `semantic-search` feature is not compiled. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::FpfSearchParams`_

## Returns

```json
{
  "query": "trust calculus",
  "mode": "keyword",
  "results": [
    {
      "id": "B.3",
      "title": "Trust Calculus",
      "score": 9.41,
      "snippet": "Trust is not binary. It is a calculus over evidence, context, and recency…",
      "path": "B. Principles > B.3 Trust Calculus"
    }
  ],
  "total": 5,
  "warning": null
}
```

On fallback:

```json
{
  "mode": "keyword",
  "warning": "semantic search requested but semantic-search feature not compiled — fell back to keyword"
}
```

## Example invocation

```json
{ "query": "trust calculus", "limit": 5 }
```

Semantic variant:

```json
{ "query": "how do agents handle uncertainty", "semantic": true }
```

## Typical sequence

1. `forgeplan_fpf_search` with the concept you need.
2. `forgeplan_fpf_section` with the top `id` — read full body.
3. Feed the section into `forgeplan_reason` for ADI reasoning with FPF grounding.

## CLI equivalent

```bash
forgeplan fpf search "trust calculus"
forgeplan fpf search "uncertainty" --semantic
```

## See also

- [`forgeplan_fpf_section`](/docs/mcp/forgeplan_fpf_section/) — fetch a section by ID.
- [`forgeplan_search`](/docs/mcp/forgeplan_search/) — search workspace artifacts (not FPF KB).
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) — ADI reasoning with optional `--fpf` grounding.
- [Methodology guide](/docs/methodology/overview/)
