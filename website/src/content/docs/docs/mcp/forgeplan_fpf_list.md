---
title: forgeplan_fpf_list
description: "List all available FPF (First Principles Framework) sections in the knowledge base."
---

Returns the complete catalogue of FPF knowledge-base sections - the hierarchical taxonomy behind Forgeplan's reasoning engine. Each section is a small, self-contained piece of the First Principles Framework (ADI cycle, trust calculus, bounded contexts, explore/investigate/exploit, etc.). `forgeplan_fpf_list` is the agent's index: browse it, decide which sections look relevant, then pull full content via `forgeplan_fpf_section` or search contents via `forgeplan_fpf_search`.

**Category**: FPF Knowledge Base

## When an agent calls this

- First call of a session when the agent needs to remember what FPF topics exist at all.
- Before `forgeplan_fpf_section` - confirm the exact ID of a section (e.g. `B.3` for Trust Calculus).
- When the user asks "does FPF have anything to say about X?" and the agent wants to scan titles before committing to a vector search.
- After `forgeplan fpf ingest` - inspect the catalogue actually loaded in this workspace; its size depends on the ingested corpus.

## What you get back

`sections` is an array of objects with `section_id`, `title` and `line_count`. `total` is its length and `_next_action` points at a real section, or suggests CLI `forgeplan fpf ingest` when empty.

This is not an array of `{id, summary, chapter}` objects, and the endpoint does not promise a fixed corpus size.

## Input parameters

_No declared input fields. Call this tool with an empty object `{}`._

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Use a returned `section_id` as the **input** `id` of `forgeplan_fpf_section`. `line_count` describes section size, not a confidence score. The catalogue does not include full bodies; read those on demand.

Success is JSON in MCP text content. This parameterless tool reads the server's default workspace, not an explicit per-call worktree. A missing FPF table or list-query failure is logged and may yield `sections: []`, `total: 0` with ingest guidance; an empty response is not proof that ingestion never occurred. There is no promised sort order or nested hierarchy in the response, and no mutation is performed.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

```
forgeplan_fpf_list
   ↓ pick interesting IDs
forgeplan_fpf_section { id: "B.3" }    ← full content of one section
   or
forgeplan_fpf_search  { query: "…" }   ← content-level search (keyword or BGE-M3)
```

## CLI equivalent

- [`forgeplan fpf list`](/docs/cli/fpf-list/) - same catalogue rendered as a terminal table.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_fpf_section`](/docs/mcp/forgeplan_fpf_section/) - fetch full body of one section
- [`forgeplan_fpf_search`](/docs/mcp/forgeplan_fpf_search/) - keyword / semantic search
- [`forgeplan_fpf_rules`](/docs/mcp/forgeplan_fpf_rules/) - rules derived from the framework
