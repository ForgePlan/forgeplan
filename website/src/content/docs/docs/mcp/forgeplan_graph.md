---
title: forgeplan_graph
description: "Generate a mermaid dependency graph of all linked artifacts. Includes explicit links and parent_epic belongs_to edges."
---

Generate a Mermaid `graph TD` of all linked artifacts in the workspace. Includes explicit typed links (`based_on`, `refines`, `supersedes`, `contradicts`, `informs`) plus implicit `belongs_to` edges from the `parent_epic` frontmatter field.

**Category**: Dashboards & Graph

## When an agent calls it

- **Visualizing architecture** — show how PRD → RFC → ADR → Evidence chains connect.
- **Impact analysis** — trace which downstream artifacts depend on a node before superseding it.
- **Documentation generation** — embed the diagram in a README or website page.
- **Review prep** — eyeball cycles or missing edges before a release.

The output is raw Mermaid markdown — render it in any Markdown viewer that supports Mermaid (GitHub, Starlight, VS Code preview, etc.).

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

```json
{
  "mermaid": "graph TD\n  EPIC-003[EPIC-003] --> PRD-035[PRD-035]\n  EPIC-003 --> PRD-039[PRD-039]\n  PRD-039 --> RFC-006[RFC-006]\n  RFC-006 --> ADR-004[ADR-004]\n  EVID-052[EVID-052] -.informs.-> PRD-039\n",
  "node_count": 184,
  "edge_count": 312
}
```

Edge styles:
- Solid (`-->`) — structural (`based_on`, `refines`, `parent_epic`).
- Dashed (`-.->`) — informational (`informs`, `weakens`, `supports`).
- Red (`==>`) — `contradicts`.

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_graph` — fetch mermaid.
2. Render it or save to `docs/graph.md`.
3. `forgeplan_order` — get topological sort for the same graph.
4. `forgeplan_blocked` — find nodes blocked by unmet prerequisites.

## CLI equivalent

```bash
forgeplan graph
```

## See also

- [`forgeplan_order`](/docs/mcp/forgeplan_order/) — topological ordering of the graph.
- [`forgeplan_blocked`](/docs/mcp/forgeplan_blocked/) — artifacts blocked by dependencies.
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) — add a new edge.
- [Methodology guide](/docs/methodology/overview/)
