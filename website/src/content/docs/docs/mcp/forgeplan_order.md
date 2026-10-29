---
title: forgeplan_order
description: "Show artifacts in topological order (dependency order). Returns ordered list, ready/blocked classification, and cycle detection. Uses structural relations only."
---

Reports the structural relation graph (`based_on`, `refines`, `supersedes`, `contradicts`), its ordering, ready/blocked classification, and cycles. The implementation builds nodes from structural edge endpoints, so isolated artifacts need not appear. Dependencies with status `active`, `deprecated`, or `superseded` count as resolved. Use `ready` and `blocked` for scheduling; do not assume `order` is a complete inventory or a verified parents-first execution plan.

**Category**: Dashboards & Graph

## When an agent calls it

- **Sprint scheduling** - work through the `ready` list first to avoid blocking yourself.
- **Release planning** - determine the activation order for a batch of draft artifacts.
- **Cycle detection** - find reference loops (e.g. two PRDs mutually `based_on` each other).
- **Import verification** - after `forgeplan_import`, sanity-check the graph is a DAG.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

The JSON text payload has `order` and `ready` as arrays of artifact ID
strings, `blocked` as entries with `id` and `blocked_by` arrays, `cycles`
as arrays of cycle-path IDs, and `_next_action`. It has no per-node
`kind`/`ready` fields or `ready_count`/`blocked_count` fields.

An illustrative fragment is `{"ready":["PRD-039"],"blocked":[{"id":"RFC-006","blocked_by":["PRD-039"]}]}`.
This is not a captured workspace result.

## Example invocation

```json
{}
```

## Typical sequence

1. Call `forgeplan_order` and inspect `cycles`, `ready`, and `blocked`.
2. Choose an ID from `ready`, not a `ready: true` field.
3. Read, implement, validate and supply evidence as required before activation.
4. Re-read the graph after the prerequisite status changes; other unresolved dependencies may still block downstream work.

## CLI equivalent

```bash
forgeplan order
```

## See also

- [`forgeplan_blocked`](/docs/mcp/forgeplan_blocked/) - focused view on blocked artifacts only.
- [`forgeplan_graph`](/docs/mcp/forgeplan_graph/) - Mermaid rendering of the same graph.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Show artifacts in topological order (dependency order). Returns ordered list, ready/blocked classification, and cycle detection. Uses structural relations only.

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
