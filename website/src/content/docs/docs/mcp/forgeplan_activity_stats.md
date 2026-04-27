---
title: forgeplan_activity_stats
description: "Aggregate the activity log by tool name — counts, error rates, p50/p95 duration, total time."
---

Reads the same activity log as [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) but
returns one row per distinct tool name with rollups: call count, error count, p50 / p95
duration, total wall time. Use it as the entry point when investigating where a session
spent its time, instead of paging through individual entries.

**Category**: Observability & Audit

## When an agent calls it

- Start of a debugging session: "which tool is hot in the last 24 h?".
- Cost / latency triage when the user reports a slow workflow.
- Pre-release sanity check: are any tools showing elevated `err_count` since the last build?
- After a marathon session — confirm the destructive-tool count matches the agent's
  mental model (no surprise `forgeplan_delete` calls).

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `since_hours` | `number` | no (default 24, max 720) | Time window in hours. `720` = 30 days. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ActivityStatsParams`_

## Returns

```json
{
  "stats": [
    {
      "tool": "forgeplan_score",
      "count": 42,
      "err_count": 0,
      "p50_ms": 110,
      "p95_ms": 240,
      "total_ms": 5180
    },
    {
      "tool": "forgeplan_search",
      "count": 18,
      "err_count": 1,
      "p50_ms": 95,
      "p95_ms": 410,
      "total_ms": 2200
    }
  ],
  "total_calls": 60,
  "total_errors": 1,
  "total_ms": 7380,
  "since_hours": 24,
  "_next_action": "60 total call(s), 1 error(s), 7380 ms total. Top by time: `forgeplan_score` ..."
}
```

Rows are ordered by `total_ms` descending (most expensive first), matching how the
`_next_action` hint surfaces the top tool.

## Example invocation

Default window (last 24 h):

```json
{}
```

Full-month rollup:

```json
{ "since_hours": 720 }
```

## Typical sequence

1. `forgeplan_activity_stats` — find the hot or erroring tool.
2. `forgeplan_activity tool=<hot>` — drill into individual calls.
3. If errors stand out — `forgeplan_activity tool=<hot> status=tool_err`.

## CLI equivalent

No direct CLI command — activity instrumentation lives in the MCP layer (PRD-055).
For ad-hoc analysis, `jq` over `.forgeplan/logs/tools-*.jsonl` produces the same
shape with more flexibility.

## See also

- [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) — entry-level drill-down
- [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) — pair with stats to reverse misfires
- [MCP overview](/docs/mcp/)
