---
title: forgeplan_activity
description: "Query recorded workspace MCP activity by time, tool and outcome."
---

Returns matching entries from `.forgeplan/logs/tools-YYYY-MM-DD.jsonl`.
Entries record tool name, argument digest, outcome and duration, not token
usage or a separate error-class field. Logging is best-effort and not fsynced:
an absent entry does not prove that an operation did not happen. Use timings
and call counts to investigate spend alongside provider usage records, not
as direct token or billing measurements.

**Category**: Observability & Audit

## When an agent calls it

- After a session interruption - "what tools did I run in the last hour?".
- Before invoking a destructive op - confirm the previous one finished and was not
  retried by mistake.
- When the user asks "where did the tokens go?" - drill into the slowest / most-used tools.
- To build a forensic trail for a `Note` after fixing a brittle workflow.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `since_hours` | `integer \| null` | no (default 24) | Rolling hours back from now, clamped to 1..=720. Despite “today-only” in captured schema prose, omission means 24 hours in the handler. |
| `tool` | `string` | no | Comma-separated tool names to filter, e.g. `"forgeplan_score,forgeplan_activate"`. |
| `status` | `string` | no | Filter by status - one of `ok`, `tool_err`, or `rpc_err`. |
| `limit` | `integer \| null` | no (default 500) | Keeps the most recent N entries; clamped to 1..=5000. |
| `workspace` | `string \| null` | no | Explicit absolute or tilde-expanded workspace path. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ActivityQueryParams`_

## Returns

The following is an illustrative partial payload, not a recorded audit.
Actual entries also carry `args_hash` and `workspace`, with optional
`client_info` and opt-in `args`. Review `warnings` for skipped corrupt data.

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

1. `forgeplan_activity_stats` - fast aggregate to find the busy / slow tools.
2. `forgeplan_activity tool=<top>` - drill into a specific tool's entries.
3. If a destructive op shows up unexpectedly: [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/).

## CLI equivalent

There is no direct CLI counterpart yet - the activity log is intentionally MCP-first
(introduced by PRD-055). The raw JSONL files at `.forgeplan/logs/tools-*.jsonl` are
human-readable as a fallback.

## See also

- [`forgeplan_activity_stats`](/docs/mcp/forgeplan_activity_stats/) - per-tool aggregates
- [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) - reverse the last destructive op
- [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) - restore a specific soft-deleted artifact
- [MCP overview](/docs/mcp/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Query the activity log — append-only JSONL record of every MCP tool invocation at .forgeplan/logs/tools-YYYY-MM-DD.jsonl. Use this to reconstruct what the agent did over a time window, attribute LLM-token spend, or audit destructive operations. Params: since_hours (default 24, max 720), tool (comma-separated names to filter), status (ok/tool_err/rpc_err), limit (default 500, max 5000 — keeps most recent).

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `limit` | no | integer \| null | Cap result set (most recent N). Default 500, max 5000. |
| `since_hours` | no | integer \| null | Time window in hours back from now (e.g. 1 = last hour, 24 = last day).<br>Omit for today-only scope. Values 1..=720 (30 days). |
| `status` | no | string \| null | Filter by status: `ok`, `tool_err`, or `rpc_err`. Omit for all. |
| `tool` | no | string \| null | Filter by tool name. Repeat with comma to match multiple:<br>`"forgeplan_score,forgeplan_activate"`. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "limit": {
      "default": null,
      "description": "Cap result set (most recent N). Default 500, max 5000.",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "since_hours": {
      "default": null,
      "description": "Time window in hours back from now (e.g. 1 = last hour, 24 = last day).\nOmit for today-only scope. Values 1..=720 (30 days).",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "status": {
      "default": null,
      "description": "Filter by status: `ok`, `tool_err`, or `rpc_err`. Omit for all.",
      "type": [
        "string",
        "null"
      ]
    },
    "tool": {
      "default": null,
      "description": "Filter by tool name. Repeat with comma to match multiple:\n`\"forgeplan_score,forgeplan_activate\"`.",
      "type": [
        "string",
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
  "title": "ActivityQueryParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
