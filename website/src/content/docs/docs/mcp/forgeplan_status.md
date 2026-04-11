---
title: forgeplan_status
description: "Show project status dashboard — total artifacts, counts by kind and status."
---

Show a compact project status dashboard — total artifacts plus counts grouped by kind and status. This is a lighter-weight alternative to `forgeplan_health` when you only need a numeric summary without blind-spot / orphan analysis.

**Category**: Dashboards & Graph

## When an agent calls it

- **Quick sanity check** — how many artifacts exist, how many are active vs draft.
- **Progress tracking across sprints** — compare counts to a previous baseline.
- **Answering "what's in this workspace?"** without triggering the full health scan.
- **Fast path** when `forgeplan_health` is too heavy (e.g. large workspaces).

If you need debts / risks / next actions, call `forgeplan_health` instead.

## Input parameters

_No input parameters. Call this tool with an empty object `{}`._

## Returns

```json
{
  "total": 184,
  "by_kind": {
    "prd": 42,
    "rfc": 18,
    "adr": 12,
    "epic": 5,
    "spec": 9,
    "evidence": 57,
    "problem": 34,
    "solution": 7,
    "note": 45,
    "refresh": 3
  },
  "by_status": {
    "draft": 12,
    "active": 98,
    "stale": 4,
    "superseded": 23,
    "deprecated": 47
  }
}
```

## Example invocation

```json
{}
```

## Typical sequence

1. `forgeplan_status` — get counts.
2. `forgeplan_list` with `kind` / `status` filter — drill down into a specific slice.
3. `forgeplan_get` — read a specific artifact.

## CLI equivalent

```bash
forgeplan status
```

## See also

- [`forgeplan_health`](/docs/mcp/forgeplan_health/) — full health report with blind spots and next actions.
- [`forgeplan_list`](/docs/mcp/forgeplan_list/) — filtered artifact listing.
- [Methodology guide](/docs/methodology/overview/)
