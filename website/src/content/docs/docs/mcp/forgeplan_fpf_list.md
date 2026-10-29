---
title: forgeplan_fpf_list
description: "List the FPF sections actually ingested in this workspace, with section_id, title, and line_count."
---

Returns the complete catalogue of FPF knowledge-base sections - the hierarchical taxonomy behind Forgeplan's reasoning engine. Each section is a small, self-contained piece of the First Principles Framework (ADI cycle, trust calculus, bounded contexts, explore/investigate/exploit, etc.). `forgeplan_fpf_list` is the agent's index: browse it, decide which sections look relevant, then pull full content via `forgeplan_fpf_section` or search contents via `forgeplan_fpf_search`.

**Category**: FPF Knowledge Base

## When an agent calls this

- First call of a session when the agent needs to remember what FPF topics exist at all.
- Before `forgeplan_fpf_section` - confirm the exact ID and title from the returned catalog.
- When the user asks "does FPF have anything to say about X?" and the agent wants to scan titles before committing to a vector search.
- After `forgeplan fpf ingest` - inspect the actual catalog; the count depends on the ingested corpus, not a fixed 204-section contract.

## What you get back

An object with `total`, `sections`, and a next-action hint. Each section summary has:

| Field | Type | Description |
|-------|------|-------------|
| `section_id` | `string` | Section ID to use with `forgeplan_fpf_section`. |
| `title` | `string` | Title from the ingested source. |
| `line_count` | `integer` | Stored section line count. |

The tool reads the auxiliary FPF table without creating or ingesting it. A workspace with no FPF table returns an empty catalog; other read failures remain errors.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

Illustrative empty-catalog response excerpt:

```json
{ "total": 0, "sections": [] }
```

Use the returned IDs and titles; this reference does not assign invented headings or summaries to upstream FPF section numbers.

## Example invocation

```json
{}
```

## Typical sequence

```
forgeplan_fpf_list
   ↓ pick interesting IDs
forgeplan_fpf_section { id: "<returned-section-id>" } ← full content of one section
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

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

List all available FPF (First Principles Framework) sections in the knowledge base.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| — | — | — | No parameters. |

<details>
<summary>JSON Schema</summary>

```json
{
  "properties": {},
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
