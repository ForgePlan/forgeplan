---
title: forgeplan_activity
description: "Query the activity log - append-only JSONL record of every MCP tool invocation at .forgeplan/logs/tools-YYYY-MM-DD.jsonl. Use this to reconstruct what the agent did over a time window, attribute LLM-token spend, or audit destructive operations. Params: since_hours (default 24, max 720), tool (comma-separated names to filter), status (ok/tool_err/rpc_err), limit (default 500, max 5000 - keeps most recent)."
---

Returns the entries in the workspace activity log
(`.forgeplan/logs/tools-YYYY-MM-DD.jsonl`) that match the given filters.
The log records tool name, argument hash, status, duration and workspace, with optional
client information and opt-in raw arguments. Use it to audit operations or rebuild a
session timeline. It does not record token usage, API cost or an error-class field;
latency and call counts are only indirect clues when investigating LLM spend.

**Category**: Observability & Audit

## When an agent calls it

- After a session interruption - "what tools did I run in the last hour?".
- Before invoking a destructive op - confirm the previous one finished and was not
  retried by mistake.
- When the user asks "where did the tokens go?" - drill into the slowest / most-used tools.
- To build a forensic trail for a `Note` after fixing a brittle workflow.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `limit` | `integer \| null` | no | Cap result set (most recent N). Default 500, max 5000. Schema default: `null`. `format`: `uint32`. `minimum`: `0`. |
| `since_hours` | `integer \| null` | no | Time window in hours back from now (e.g. 1 = last hour, 24 = last day). Omit for today-only scope. Values 1..=720 (30 days). Schema default: `null`. `format`: `uint32`. `minimum`: `0`. |
| `status` | `string \| null` | no | Filter by status: `ok`, `tool_err`, or `rpc_err`. Omit for all. Schema default: `null`. |
| `tool` | `string \| null` | no | Filter by tool name. Repeat with comma to match multiple: `"forgeplan_score,forgeplan_activate"`. Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON serialized inside MCP text content. The following values are illustrative.

```json
{
  "entries": [
    {
      "ts": "2026-04-26T10:14:22Z",
      "tool": "forgeplan_score",
      "args_hash": "a1b2c3d4e5f6",
      "workspace": "/work/project/.forgeplan",
      "status": "ok",
      "duration_ms": 142
    }
  ],
  "total_scanned": 312,
  "returned": 1,
  "warnings": [],
  "since_hours": 24,
  "_next_action": "1 entries in window. Busiest: `forgeplan_score`. Per-tool breakdown: `forgeplan_activity_stats`."
}
```

The `_next_action` hint nudges the agent toward the right follow-up
(`forgeplan_activity_stats` for aggregates, or a narrower `tool=` filter).

The handler defaults to the last **24 hours**, despite the input-schema description's
today-only wording. It clamps `since_hours` to 1–720 and `limit` to 1–5000.
Returned entries remain chronological after keeping the most recent N.
`total_scanned` counts lines read before filtering, including malformed lines.
`warnings` reports parse failures; missing daily files contribute no entries.
`client_info` (`name`, optional `version`) and `args` are omitted unless recorded.
An unknown status simply matches no entries. Workspace-resolution failures are tool
errors; log-read failures are JSON-RPC errors, not successful empty results.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_activity_stats` - fast aggregate to find the busy / slow tools.
2. `forgeplan_activity tool=<top>` - drill into a specific tool's entries.
3. If a destructive op shows up unexpectedly: [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/).

## CLI equivalent

```bash
forgeplan activity --since-hours 24 --status tool_err --limit 50 --json
```

[`forgeplan activity`](/docs/cli/activity/) reads the same JSONL logs. The CLI option is `--since-hours`, corresponding to MCP `since_hours`.

## See also

- [`forgeplan_activity_stats`](/docs/mcp/forgeplan_activity_stats/) - per-tool aggregates
- [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) - reverse the last destructive op
- [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) - restore a specific soft-deleted artifact
- [MCP overview](/docs/mcp/)
