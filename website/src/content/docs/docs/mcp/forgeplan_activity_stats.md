---
title: forgeplan_activity_stats
description: "Aggregate statistics from the activity log grouped by tool name: count, error count, p50/p95 duration, total time. Use to attribute LLM-token spend and identify slow tools. Params: since_hours (default 24, max 720)."
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
- After a marathon session - confirm the destructive-tool count matches the agent's
  mental model (no surprise `forgeplan_delete` calls).

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `since_hours` | `integer \| null` | no | Time window in hours. Default 24. Schema default: `null`. `format`: `uint32`. `minimum`: `0`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON inside MCP text content. Values below are illustrative, not measured results.

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
  "_next_action": "60 call(s), 1 error(s), 7380 ms total. Drill into busiest: `forgeplan_activity tool=\"forgeplan_score\"`."
}
```

Rows are ordered by `total_ms` descending (most expensive first), matching how the
`_next_action` hint surfaces the top tool.

The handler defaults to 24 hours and clamps `since_hours` to 1–720. Unlike
`forgeplan_activity`, it imposes no entry limit. `err_count` counts entries whose
status is not `ok`; p50/p95 use nearest-rank percentiles of recorded durations.
Totals are sums of tool durations, not session elapsed time or token/cost measurements.
Malformed JSONL lines are skipped by the query layer, but this response does not
include its warnings; use `forgeplan_activity` to inspect them. No log files yields
empty `stats` and zero totals. Workspace-resolution failures are tool errors;
log-read failures are JSON-RPC errors.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_activity_stats` - find the hot or erroring tool.
2. `forgeplan_activity tool=<hot>` - drill into individual calls.
3. If errors stand out - `forgeplan_activity tool=<hot> status=tool_err`.

## CLI equivalent

```bash
forgeplan activity-stats --since-hours 24 --json
```

[`forgeplan activity-stats`](/docs/cli/activity-stats/) aggregates the same activity logs.

## See also

- [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) - entry-level drill-down
- [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) - pair with stats to reverse misfires
- [MCP overview](/docs/mcp/)
