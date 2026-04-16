---
title: forgeplan_blocked
description: "Show blocked artifacts and their unmet dependencies. Only draft artifacts block — deprecated and superseded are considered resolved. Uses structural relations only (based_on, refines, supersedes, contradicts)."
---

Show artifacts that are blocked by unmet structural dependencies. An artifact is "blocked" when it references another artifact (via `based_on`, `refines`, `supersedes`, or `contradicts`) that is still in `draft` state — meaning the prerequisite decision has not been activated yet. `deprecated` and `superseded` prerequisites are treated as resolved, not blockers.

**Category**: Dashboards & Graph

## When an agent calls it

- **Sprint planning** — determine which artifacts are ready to work on vs waiting on upstream decisions.
- **Before activation** — confirm an artifact has no unmet prerequisites before calling `forgeplan_activate`.
- **Bottleneck diagnosis** — identify the upstream draft that is blocking many downstream artifacts.
- **Session start** — quickly see "what's ready to pick up" after restoring context.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | no | Artifact ID to check. If omitted, lists all blocked artifacts. |

_Schema source: `crates/forgeplan-mcp/src/types.rs::BlockedParams`_

## Returns

```json
{
  "blocked": [
    {
      "id": "RFC-006",
      "kind": "rfc",
      "status": "draft",
      "blocked_by": [
        { "id": "PRD-039", "status": "draft", "relation": "based_on" }
      ]
    }
  ],
  "total_blocked": 1
}
```

## Example invocation

```json
{}
```

Or for a single artifact:

```json
{ "id": "RFC-006" }
```

## Typical sequence

1. `forgeplan_blocked` — list blocked artifacts.
2. `forgeplan_get` on the upstream `blocked_by` — check what it needs.
3. `forgeplan_activate` the upstream once validation passes.
4. `forgeplan_blocked` again — confirm downstream is now unblocked.

## CLI equivalent

```bash
forgeplan blocked
forgeplan blocked RFC-006
```

## See also

- [`forgeplan_order`](/docs/mcp/forgeplan_order/) — full topological sort of the dependency graph.
- [`forgeplan_graph`](/docs/mcp/forgeplan_graph/) — visualize the graph.
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) — unblock downstream by activating a draft.
- [Methodology guide](/docs/methodology/overview/)
