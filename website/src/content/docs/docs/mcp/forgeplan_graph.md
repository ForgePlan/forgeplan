---
title: forgeplan_graph
description: "Generate a mermaid dependency graph of all linked artifacts. Includes explicit links and parent_epic belongs_to edges. Issue #287 Phase F: pass `brownfield_only: true` to filter to UC/GLOS/INV/SCEN/HYP/DM edges only."
---

Generate a Mermaid `graph LR` of linked artifacts in the workspace. Includes explicit typed links (`based_on`, `refines`, `supersedes`, `contradicts`, `informs`) plus implicit child-to-parent `belongs_to` edges from `parent_epic`. Domain Model Composition sections add clusters and their members; this is not an inventory of every isolated artifact.

**Category**: Dashboards & Graph

## When an agent calls it

- **Visualizing architecture** - show how PRD → RFC → ADR → Evidence chains connect.
- **Impact analysis** - trace which downstream artifacts depend on a node before superseding it.
- **Documentation generation** - embed the diagram in a README or website page.
- **Review prep** - eyeball cycles or missing edges before a release.

The response is JSON in MCP `content[0].text`; extract `mermaid` and render that string in a Mermaid-capable viewer.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `brownfield_only` | `boolean \| null` | no | Filter edges to brownfield kinds (UC / GLOS / INV / SCEN / HYP / DM) only. Defaults to false (show all edges including pipeline). Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The object contains `mermaid` and `_next_action`, not node/edge arrays or `node_count`/`edge_count`. This tool renders a report and does not add links or mutate artifacts. The hint's edge count is computed before the optional brownfield filter.

Illustrative empty-graph response:

```json
{
  "mermaid": "graph LR\n    %% No links found between artifacts\n",
  "_next_action": "No links yet — isolated artifacts. Inspect orphans: `forgeplan_blindspots`."
}
```

Edges normally use labelled solid arrows, including `informs` and `contradicts`. `belongs_to` uses an unlabelled solid arrow; `references` uses a dashed arrow. Brownfield edges can receive additional `linkStyle` overrides, and hypothesis nodes are coloured by state. Do not interpret every rendered edge as a blocking dependency: `order` and `blocked` use structural relations only.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_graph` - fetch mermaid.
2. Render it or save to `docs/graph.md`.
3. `forgeplan_order` - get topological sort for the same graph.
4. `forgeplan_blocked` - find nodes blocked by unmet prerequisites.

## CLI equivalent

```bash
forgeplan graph
```

## See also

- [`forgeplan_order`](/docs/mcp/forgeplan_order/) - topological ordering of the graph.
- [`forgeplan_blocked`](/docs/mcp/forgeplan_blocked/) - artifacts blocked by dependencies.
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) - add a new edge.
- [Methodology guide](/docs/methodology/overview/)
