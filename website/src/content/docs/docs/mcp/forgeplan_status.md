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

_No input parameters. Call this tool with an empty object `{}`._

## Returns

Illustrative response excerpt, not a workspace measurement. The grouped counts are arrays of objects, not maps.

```json
{
  "project": "example",
  "workspace": "/abs/path/.forgeplan",
  "total": 2,
  "by_kind": [
    { "kind": "evidence", "count": 1 },
    { "kind": "prd", "count": 1 }
  ],
  "by_status": [
    { "status": "active", "count": 1 },
    { "status": "draft", "count": 1 }
  ]
}
```

## Example invocation

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

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Show project status dashboard — total artifacts, counts by kind and status.

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
