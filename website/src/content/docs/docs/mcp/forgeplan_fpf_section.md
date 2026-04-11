---
title: forgeplan_fpf_section
description: "Get full content of a specific FPF section by ID (e.g. 'B.3', 'C.2.2', 'A.1')."
---

Fetch the full markdown content of a specific FPF (First Principles Framework) section by its stable ID — e.g. `B.3` (Trust Calculus), `B.5` (ADI Cycle), `C.2.2`, `A.1`. Returns the complete section body plus linked sibling / parent / child section IDs for navigation.

**Category**: FPF Knowledge Base

## When an agent calls it

- **After `forgeplan_fpf_search`** — read the full text of the top hit.
- **Direct lookup** — when you already know the section ID from prior context or a citation.
- **Navigation** — follow `parent` / `children` links to traverse the FPF tree.
- **Context grounding** — pull a specific principle into an ADI reasoning prompt.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | FPF section ID, e.g. `"B.3"`, `"C.2.2"`, `"A.1"`. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::FpfSectionParams`_

## Returns

```json
{
  "id": "B.3",
  "title": "Trust Calculus",
  "path": "B. Principles > B.3 Trust Calculus",
  "body": "# B.3 Trust Calculus\n\nTrust is not binary. It is a function of…",
  "parent": "B",
  "children": ["B.3.1", "B.3.2"],
  "siblings": ["B.1", "B.2", "B.4", "B.5"],
  "word_count": 342
}
```

If the ID is unknown:

```json
{
  "error": "section not found: B.99",
  "suggestions": ["B.9", "B.3"]
}
```

## Example invocation

```json
{ "id": "B.3" }
```

## Typical sequence

1. `forgeplan_fpf_search "trust"` — discover `B.3` is the top hit.
2. `forgeplan_fpf_section { "id": "B.3" }` — read full body.
3. `forgeplan_fpf_section { "id": "B.3.1" }` — walk into a child section.
4. Use the content as FPF grounding for `forgeplan_reason`.

## CLI equivalent

```bash
forgeplan fpf section B.3
```

## See also

- [`forgeplan_fpf_search`](/docs/mcp/forgeplan_fpf_search/) — discover sections by query.
- [`forgeplan_reason`](/docs/mcp/forgeplan_reason/) — ADI reasoning with FPF context.
- [Methodology guide](/docs/methodology/overview/)
