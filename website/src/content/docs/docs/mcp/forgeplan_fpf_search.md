---
title: forgeplan_fpf_search
description: "Search the ingested FPF knowledge base by keyword or cached BGE-M3 vectors, with explicit fallback warnings."
---

Search the ingested **FPF (First Principles Framework) knowledge base**. The available sections depend on the corpus explicitly ingested into this workspace; use `forgeplan_fpf_list` to discover them. `semantic: true` requests BGE-M3 vector search when the build and cached model support it; otherwise keyword search answers with a warning.

**Category**: FPF Knowledge Base

## When an agent calls it

- **Looking up a principle** - "what does FPF say about trust calculus?" → agents often hit B.3.
- **Grounding ADI reasoning** - before generating hypotheses, fetch relevant FPF context.
- **Decision support** - `fpf_search "explore exploit"` surfaces the exploration/exploitation balance rules.
- **Onboarding** - help a new agent discover FPF vocabulary quickly.

This reads the auxiliary FPF table, not workspace artifacts and not the same pipeline as `forgeplan_search`. In the selected local build, reads never download model weights. The captured catalog below still mentions a first-call download; the handler instead uses a verified cached snapshot and falls back to keyword when that cache is unavailable. Obtain weights explicitly before querying if vector search is needed.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `query` | `string` | yes | Non-empty after trimming for validation; the implementation limits UTF-8 length to 8192 bytes, despite the catalog's “chars” wording. |
| `limit` | `integer \| null` | no (default: `5`, cap: `50`) | Maximum returned hits; the handler allows zero. |
| `semantic` | `boolean \| null` | no (default: `false`) | Request cached vector search; inspect `warning` for keyword fallback. |
| `workspace` | `string \| null` | no | Explicit absolute or tilde-expanded workspace path. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::FpfSearchParams`_

## Returns

The parsed payload contains `query`, `semantic`, `count`, `results`,
`warning`, and `_next_action`. Each hit has `id`, `section_id`, `title`,
`snippet`, and `line_count`; there are no `score` or `path` fields.
Use `section_id` for `forgeplan_fpf_section`.

`semantic` echoes the requested flag in this handler, including when it
falls back. A non-null `warning` identifies fallback; do not infer actual
vector execution from `semantic: true` alone. The response has no `mode`
or `total` field. `_next_action` notes when more matches exist than the cap
returned. A missing FPF table is an error requiring explicit `fpf ingest`,
not an empty successful search.

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
2. Pass the top hit's `section_id` to `forgeplan_fpf_section` to read its body.
3. Feed the section into `forgeplan_reason` for ADI reasoning with FPF grounding.

## CLI equivalent

```bash
forgeplan fpf search "trust calculus"
forgeplan fpf search "uncertainty" --semantic
```

## See also

- [`forgeplan_fpf_section`](/docs/mcp/forgeplan_fpf_section/) - fetch a section by ID.
- [`forgeplan_search`](/docs/mcp/forgeplan_search/) - search workspace artifacts (not FPF KB).
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) - ADI reasoning with optional `--fpf` grounding.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Search FPF (First Principles Framework) knowledge base. Default is keyword search. Pass `semantic: true` for vector similarity search via BGE-M3 embeddings (requires the `semantic-search` build feature). When `semantic: true` but the feature is not compiled in, the query gracefully falls back to keyword search and the response includes a `warning` field. Note: the first invocation with `semantic: true` may take several minutes if the BGE-M3 model needs to be downloaded (~2.1 GB). Params: query (required, 1..=8192 chars), limit (default 5, max 50), semantic (default false).

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `limit` | no | integer \| null | Max results (default 5, max 50) |
| `query` | yes | string | Search query — keyword or semantic depending on `semantic` flag |
| `semantic` | no | boolean \| null | Use semantic (vector) search instead of keyword. Requires `semantic-search`<br>feature at build time. When the feature is not compiled in, the query<br>gracefully falls back to keyword search and the response includes a<br>`warning` field explaining why. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Exposed for integration test harness in tests/fpf_search_handler.rs.\nFields remain pub(crate) — only the struct itself is visible externally.\n`#[doc(hidden)]` because this is unstable test infrastructure, not a\nsupported public API (Sprint 13.7 hotfix re-audit M3).",
  "properties": {
    "limit": {
      "default": null,
      "description": "Max results (default 5, max 50)",
      "format": "uint",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "query": {
      "description": "Search query — keyword or semantic depending on `semantic` flag",
      "type": "string"
    },
    "semantic": {
      "default": null,
      "description": "Use semantic (vector) search instead of keyword. Requires `semantic-search`\nfeature at build time. When the feature is not compiled in, the query\ngracefully falls back to keyword search and the response includes a\n`warning` field explaining why.",
      "type": [
        "boolean",
        "null"
      ]
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
  "title": "FpfSearchParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
