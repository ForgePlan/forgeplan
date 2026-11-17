---
title: forgeplan_order
description: "Show artifacts in topological order (dependency order). Returns ordered list, ready/blocked classification, and cycle detection. Uses structural relations only."
---

Returns a topological traversal of structural edges (`based_on`, `refines`, `supersedes`, `contradicts`) with separate ready/blocked classifications and cycle paths. **Released ordering limitation:** although `from` means “depends on `to`”, the traversal emits `from` before `to`; do not treat `order` as prerequisite-first execution order. Use the separate `ready` list and inspect dependencies.

**Category**: Dashboards & Graph

## When an agent calls it

- **Sprint scheduling** - work through the `ready` list first to avoid blocking yourself.
- **Release planning** - determine the activation order for a batch of draft artifacts.
- **Cycle detection** - find reference loops (e.g. two PRDs mutually `based_on` each other).
- **Import verification** - after `forgeplan_import`, sanity-check the graph is a DAG.

## Input parameters

_No declared input fields. Call this tool with an empty object `{}`._

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Illustrative response for a single `RFC-006 based_on PRD-039` edge with a draft PRD:

```json
{
  "order": ["RFC-006", "PRD-039"],
  "ready": ["PRD-039"],
  "blocked": [{"id": "RFC-006", "blocked_by": ["PRD-039"]}],
  "cycles": [],
  "_next_action": "Illustrative guidance to work a ready artifact."
}
```

Success is JSON in MCP text content. `order` and `ready` contain ID strings, not artifact objects; there are no count fields. `cycles` contains arrays of ID paths, for example `["PRD-042","PRD-043","PRD-042"]`. Cyclic nodes can be absent from `order`, `ready` and `blocked`.

Only endpoints of structural edges enter the graph; isolated artifacts and informational-only links are omitted. A dependency is resolved only when its target record is `active`, `deprecated` or `superseded`; `stale` and missing targets are unresolved. The tool reads the server's default workspace and has no per-call `workspace` argument. Workspace/store failures return an error, not an empty successful graph.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_order` - get the full ordering.
2. Pick an ID from the separate `ready` array, not the first item in `order`.
3. `forgeplan_get` → `forgeplan_validate` → `forgeplan_activate`.
4. Re-run `forgeplan_order` - downstream nodes should now become ready.

## CLI equivalent

```bash
forgeplan order
```

## See also

- [`forgeplan_blocked`](/docs/mcp/forgeplan_blocked/) - focused view on blocked artifacts only.
- [`forgeplan_graph`](/docs/mcp/forgeplan_graph/) - Mermaid rendering of the same graph.
- [Methodology guide](/docs/methodology/overview/)
