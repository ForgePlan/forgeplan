---
title: forgeplan_activity_stats
description: "Aggregate the activity log by tool name - counts, error rates, p50/p95 duration, total time."
---

Reads the same activity log as [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) but
returns one row per distinct tool name with rollups: call count, error count, p50 / p95
duration, and the sum of recorded call durations. Overlapping calls can make that sum
larger than session elapsed time; these fields do not measure tokens or monetary cost.

**Category**: Observability & Audit

## When an agent calls it

- Start of a debugging session: "which tool is hot in the last 24 h?".
- Cost / latency triage when the user reports a slow workflow.
- Pre-release sanity check: are any tools showing elevated `err_count` since the last build?
- After a marathon session - confirm the destructive-tool count matches the agent's
  mental model (no surprise `forgeplan_delete` calls).

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `since_hours` | `integer` | no (default 24) | Time window in hours, clamped to `1..=720` by the handler. `720` = 30 days. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ActivityStatsParams`_

## Returns
Illustrative response, not a measured session:


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

1. `forgeplan_activity_stats` - find the hot or erroring tool.
2. `forgeplan_activity tool=<hot>` - drill into individual calls.
3. If errors stand out - `forgeplan_activity tool=<hot> status=tool_err`.

## CLI equivalent

`forgeplan activity-stats --since-hours 720 --json` queries the activity log from the CLI.
For ad-hoc analysis, inspect `.forgeplan/logs/tools-*.jsonl`; neither interface supplies
token counts or a billing total.

## See also

- [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) - entry-level drill-down
- [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) - pair with stats to reverse misfires
- [MCP overview](/docs/mcp/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Aggregate statistics from the activity log grouped by tool name: count, error count, p50/p95 duration, total time. Use to attribute LLM-token spend and identify slow tools. Params: since_hours (default 24, max 720).

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `since_hours` | no | integer \| null | Time window in hours. Default 24. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "since_hours": {
      "default": null,
      "description": "Time window in hours. Default 24.",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "ActivityStatsParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
