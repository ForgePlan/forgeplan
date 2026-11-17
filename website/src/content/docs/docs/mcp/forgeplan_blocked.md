---
title: forgeplan_blocked
description: "Show unresolved structural dependencies. Active, deprecated and superseded targets are resolved; draft, stale and missing targets can block."
---

Shows unmet structural dependencies (`based_on`, `refines`, `supersedes`, `contradicts`): `from` depends on `to`. Targets with status `active`, `deprecated` or `superseded` are resolved. Other targets, including `stale` or missing records, remain unresolved; this is not strictly a draft-only check. Informational links do not block.

**Category**: Dashboards & Graph

## When an agent calls it

- **Sprint planning** - determine which artifacts are ready to work on vs waiting on upstream decisions.
- **Before activation** - confirm an artifact has no unmet prerequisites before calling `forgeplan_activate`.
- **Bottleneck diagnosis** - identify the upstream draft that is blocking many downstream artifacts.
- **Session start** - quickly see "what's ready to pick up" after restoring context.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string \| null` | no | Optional artifact ID to check. If omitted, shows all blocked artifacts. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Illustrative JSON payload in MCP text content:

```json
{
  "blocked": [{"id": "RFC-006", "blocked_by": ["PRD-039"]}],
  "ready_count": 1,
  "blocked_count": 1,
  "cycles": [],
  "_next_action": "Illustrative guidance to inspect dependencies."
}
```

`blocked_by` contains ID strings, not status/relation objects. Workspace mode only considers nodes present in structural edges; isolated artifacts are absent. Cyclic nodes are reported in `cycles` and may be absent from the ready/blocked counts. With `id`, the handler uppercases it, checks direct outgoing dependencies and returns the same shape with counts 0/1 and `cycles: []`. It does not verify that the requested artifact exists, so an unknown ID can appear unblocked. Use `forgeplan_get` to establish existence; this is a read-only graph check, not activation authorization.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_blocked` - list blocked artifacts.
2. `forgeplan_get` on the upstream `blocked_by` - check what it needs.
3. `forgeplan_activate` the upstream once validation passes.
4. `forgeplan_blocked` again - confirm downstream is now unblocked.

## CLI equivalent

```bash
forgeplan blocked
forgeplan blocked RFC-006
```

## See also

- [`forgeplan_order`](/docs/mcp/forgeplan_order/) - full topological sort of the dependency graph.
- [`forgeplan_graph`](/docs/mcp/forgeplan_graph/) - visualize the graph.
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - unblock downstream by activating a draft.
- [Methodology guide](/docs/methodology/overview/)
