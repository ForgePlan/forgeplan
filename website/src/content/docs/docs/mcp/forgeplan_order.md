---
title: forgeplan_order
description: "Show artifacts in topological order (dependency order). Returns ordered list, ready/blocked classification, and cycle detection. Uses structural relations only."
---

Return all artifacts in topological order — parents before children, based purely on structural relations (`based_on`, `refines`, `supersedes`, `contradicts`). Each node is classified as `ready` (no unmet prerequisites) or `blocked`. Cycles in the graph are detected and reported separately.

**Category**: Dashboards & Graph

## When an agent calls it

- **Sprint scheduling** — work through the `ready` list first to avoid blocking yourself.
- **Release planning** — determine the activation order for a batch of draft artifacts.
- **Cycle detection** — find reference loops (e.g. two PRDs mutually `based_on` each other).
- **Import verification** — after `forgeplan_import`, sanity-check the graph is a DAG.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

```json
{
  "order": [
    { "id": "EPIC-003", "kind": "epic", "ready": true },
    { "id": "PRD-039", "kind": "prd", "ready": true },
    { "id": "RFC-006", "kind": "rfc", "ready": false, "blocked_by": ["PRD-039"] },
    { "id": "ADR-004", "kind": "adr", "ready": false, "blocked_by": ["RFC-006"] }
  ],
  "ready_count": 2,
  "blocked_count": 2,
  "cycles": []
}
```

If cycles exist, they're reported:

```json
{
  "order": [...],
  "cycles": [
    ["PRD-042", "PRD-043", "PRD-042"]
  ]
}
```

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_order` — get the full ordering.
2. Pick the first `ready: true` node.
3. `forgeplan_get` → `forgeplan_validate` → `forgeplan_activate`.
4. Re-run `forgeplan_order` — downstream nodes should now become ready.

## CLI equivalent

```bash
forgeplan order
```

## See also

- [`forgeplan_blocked`](/docs/mcp/forgeplan_blocked/) — focused view on blocked artifacts only.
- [`forgeplan_graph`](/docs/mcp/forgeplan_graph/) — Mermaid rendering of the same graph.
- [Methodology guide](/docs/methodology/overview/)
