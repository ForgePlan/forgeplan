---
title: forgeplan_status
description: "Show project status dashboard - total artifacts, counts by kind and status."
---

Show a compact project status dashboard - total artifacts plus counts grouped by kind and status. This is a lighter-weight alternative to `forgeplan_health` when you only need a numeric summary without blind-spot / orphan analysis.

**Category**: Dashboards & Graph

## When an agent calls it

- **Quick sanity check** - how many artifacts exist, how many are active vs draft.
- **Progress tracking across sprints** - compare counts to a previous baseline.
- **Answering "what's in this workspace?"** without triggering the full health scan.
- **Fast path** when `forgeplan_health` is too heavy (e.g. large workspaces).

If you need debts / risks / next actions, call `forgeplan_health` instead.

## Input parameters

_No declared input fields. Call this tool with an empty object `{}`._

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON inside MCP text content. Illustrative values:

```json
{
  "project": "billing-service",
  "workspace": "/work/billing-service/.forgeplan",
  "total": 3,
  "by_kind": [
    { "kind": "note", "count": 1 },
    { "kind": "prd", "count": 2 }
  ],
  "by_status": [
    { "status": "active", "count": 1 },
    { "status": "draft", "count": 2 }
  ],
  "_next_action": "2 drafts pending. List them: `forgeplan_list status=draft`."
}
```

`by_kind` and `by_status` are arrays of `{kind, count}` and `{status, count}`,
not maps. Each is sorted by its label; absent groups are omitted. An empty
workspace returns `total: 0` and empty arrays. These are stored artifact statuses,
not evidence freshness computed by health. The hint recommends routing for an
empty workspace, listing drafts when drafts outnumber active artifacts, or health otherwise.

There is no `workspace` input or `resolved_via` / `resolved_workspace` trace here.
The `workspace` output identifies the selected directory. Resolution failures are
tool errors; configuration and store-read failures are JSON-RPC errors.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_status` - get counts.
2. `forgeplan_list` with `kind` / `status` filter - drill down into a specific slice.
3. `forgeplan_get` - read a specific artifact.

## CLI equivalent

```bash
forgeplan status
```

## See also

- [`forgeplan_health`](/docs/mcp/forgeplan_health/) - full health report with blind spots and next actions.
- [`forgeplan_list`](/docs/mcp/forgeplan_list/) - filtered artifact listing.
- [Methodology guide](/docs/methodology/overview/)
