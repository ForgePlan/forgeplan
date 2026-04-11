---
title: forgeplan_fpf_list
description: "List every FPF (First Principles Framework) knowledge-base section. Returns the full catalogue of 204 sections as a flat table of {id, title, summary} so the agent can pick what to fetch next via forgeplan_fpf_section."
---

Returns the complete catalogue of FPF knowledge-base sections — the hierarchical taxonomy behind Forgeplan's reasoning engine. Each section is a small, self-contained piece of the First Principles Framework (ADI cycle, trust calculus, bounded contexts, explore/investigate/exploit, etc.). `forgeplan_fpf_list` is the agent's index: browse it, decide which sections look relevant, then pull full content via `forgeplan_fpf_section` or search contents via `forgeplan_fpf_search`.

**Category**: FPF Knowledge Base

## When an agent calls this

- First call of a session when the agent needs to remember what FPF topics exist at all.
- Before `forgeplan_fpf_section` — confirm the exact ID of a section (e.g. `B.3` for Trust Calculus).
- When the user asks "does FPF have anything to say about X?" and the agent wants to scan titles before committing to a vector search.
- As a sanity check after `forgeplan fpf ingest` — the list should return ~204 sections on a fresh ingest.

## What you get back

A flat array of sections, each with:

| Field | Type | Description |
|-------|------|-------------|
| `id` | `string` | Section ID, e.g. `A.1`, `B.3`, `C.2.2`. |
| `title` | `string` | Human-readable title. |
| `summary` | `string` | 1–2 sentence abstract of the section. |
| `chapter` | `string` | Parent chapter (`A`, `B`, `C`, …). |

The list is ordered by ID so related sections sit next to each other.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

```json
{
  "total": 204,
  "sections": [
    { "id": "A.1", "chapter": "A", "title": "Why First Principles",
      "summary": "Motivation for the framework; contrast with heuristic reasoning." },
    { "id": "B.3", "chapter": "B", "title": "Trust Calculus",
      "summary": "How evidence decays, congruence levels, R_eff weakest-link rule." },
    { "id": "C.2.2", "chapter": "C", "title": "Explore / Investigate / Exploit",
      "summary": "Three action buckets keyed to R_eff thresholds." }
  ]
}
```

## Example invocation

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

- [`forgeplan fpf list`](/docs/cli/fpf-list/) — same catalogue rendered as a terminal table.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_fpf_section`](/docs/mcp/forgeplan_fpf_section/) — fetch full body of one section
- [`forgeplan_fpf_search`](/docs/mcp/forgeplan_fpf_search/) — keyword / semantic search
- [`forgeplan_fpf_rules`](/docs/mcp/forgeplan_fpf_rules/) — rules derived from the framework
