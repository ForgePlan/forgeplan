---
title: forgeplan_graph
description: "Generate a mermaid dependency graph of all linked artifacts. Includes explicit links and parent_epic belongs_to edges."
---

Generate a Mermaid `graph LR` of linked artifacts in the workspace. Includes explicit typed links plus implicit `belongs_to` edges from each child to its `parent_epic`.

**Category**: Dashboards & Graph

## When an agent calls it

- **Visualizing architecture** - show how PRD → RFC → ADR → Evidence chains connect.
- **Impact analysis** - trace which downstream artifacts depend on a node before superseding it.
- **Documentation generation** - embed the diagram in a README or website page.
- **Review prep** - eyeball cycles or missing edges before a release.

The JSON response's `mermaid` field contains Mermaid source. Render that string with a Mermaid-capable viewer; the response itself is not raw Markdown.

## Input parameters

Call with `{}` for the default graph, or supply optional nullable `brownfield_only` and `workspace` as described in the registered schema below.

## Returns

Illustrative response excerpt for an empty graph. The handler does not return `node_count` or `edge_count`.

```json
{
  "mermaid": "graph LR\n    %% No links found between artifacts\n"
}
```

Edge styles in the renderer:
- `references` uses a dashed arrow.
- `belongs_to` uses an unlabelled solid arrow; other named relations, including `informs` and `contradicts`, use labelled solid arrows.
- `covers` has a blue stroke; `triangulates` takes the hypothesis-state colour. `contradicts` does not automatically use a red `==>` arrow.

## Example invocation

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

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Generate a mermaid dependency graph of all linked artifacts. Includes explicit links and parent_epic belongs_to edges. Issue #287 Phase F: pass `brownfield_only: true` to filter to UC/GLOS/INV/SCEN/HYP/DM edges only.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `brownfield_only` | no | boolean \| null | Filter edges to brownfield kinds (UC / GLOS / INV / SCEN / HYP /<br>DM) only. Defaults to false (show all edges including pipeline). |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "description": "Issue #287 Phase F (W4): optional filter for the `forgeplan_graph`\nMCP tool. When `brownfield_only` is `true`, the rendered mermaid\ngraph drops edges whose endpoints aren't both brownfield kinds\n(UC/GLOS/INV/SCEN/HYP/DM). Pipeline-only callers (no params, or\n`brownfield_only: false`) get the legacy full-graph rendering.\n\nDefault value is `false`; zero-arg callers (existing JSON-RPC\nclients that pass `{}`) stay valid via `#[serde(default)]`.",
  "properties": {
    "brownfield_only": {
      "default": null,
      "description": "Filter edges to brownfield kinds (UC / GLOS / INV / SCEN / HYP /\nDM) only. Defaults to false (show all edges including pipeline).",
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
  "title": "GraphParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
