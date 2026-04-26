---
title: forgeplan_activity
description: "Query the activity log — append-only JSONL record of every MCP tool invocation."
---

Returns the entries in the workspace activity log
(`.forgeplan/logs/tools-YYYY-MM-DD.jsonl`) that match the given filters. Forgeplan
logs every MCP tool call — tool name, arguments digest, status, duration, error class —
so the agent can reconstruct what happened without trusting fallible memory. Use it to
attribute LLM-token spend, audit destructive operations, or rebuild a session timeline
after an interruption.

**Category**: Observability & Audit

## When an agent calls it

- After a session interruption — "what tools did I run in the last hour?".
- Before invoking a destructive op — confirm the previous one finished and was not
  retried by mistake.
- When the user asks "where did the tokens go?" — drill into the slowest / most-used tools.
- To build a forensic trail for a `Note` after fixing a brittle workflow.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `since_hours` | `number` | no (default 24, max 720) | Time window in hours back from now. `1` = last hour, `720` = last 30 days. |
| `tool` | `string` | no | Comma-separated tool names to filter, e.g. `"forgeplan_score,forgeplan_activate"`. |
| `status` | `string` | no | Filter by status — one of `ok`, `tool_err`, or `rpc_err`. |
| `limit` | `number` | no (default 500, max 5000) | Cap result set; keeps the most recent N entries. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ActivityQueryParams`_

## Returns

```json
{
  "entries": [
    {
      "ts": "2026-04-26T10:14:22Z",
      "tool": "forgeplan_score",
      "status": "ok",
      "duration_ms": 142
    }
  ],
  "total_scanned": 312,
  "returned": 1,
  "warnings": [],
  "since_hours": 24,
  "_next_action": "1 entries in window. Busiest tool: `forgeplan_score`. ..."
}
```

The `_next_action` hint nudges the agent toward the right follow-up
(`forgeplan_activity_stats` for aggregates, or a narrower `tool=` filter).

## Example invocation

Last hour of work:

```json
{ "since_hours": 1 }
```

All destructive ops in the last week:

```json
{ "since_hours": 168, "tool": "forgeplan_delete,forgeplan_supersede,forgeplan_deprecate" }
```

Errors only:

```json
{ "status": "tool_err", "limit": 50 }
```

## Typical sequence

1. `forgeplan_activity_stats` — fast aggregate to find the busy / slow tools.
2. `forgeplan_activity tool=<top>` — drill into a specific tool's entries.
3. If a destructive op shows up unexpectedly: [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/).

## CLI equivalent

There is no direct CLI counterpart yet — the activity log is intentionally MCP-first
(introduced by PRD-055). The raw JSONL files at `.forgeplan/logs/tools-*.jsonl` are
human-readable as a fallback.

## See also

- [`forgeplan_activity_stats`](/docs/mcp/forgeplan_activity_stats/) — per-tool aggregates
- [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) — reverse the last destructive op
- [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) — restore a specific soft-deleted artifact
- [MCP overview](/docs/mcp/)
