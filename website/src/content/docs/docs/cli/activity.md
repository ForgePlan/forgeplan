---
title: forgeplan activity
description: "Query the activity log - append-only JSONL record of every MCP tool invocation. Use to reconstruct what the agent did, attribute spend, or audit destructive operations."
---

`forgeplan activity` reads recorded MCP tool calls from daily `.forgeplan/logs/tools-YYYY-MM-DD.jsonl` files: tool name, arguments, status (`ok`, `tool_err`, or `rpc_err`), and duration. It does not record every CLI command or every action an agent takes. Use it to answer "which recorded calls happened in the last hour?"

This is the CLI version of [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) (the MCP tool); both read the same log files.

## When to use

- A session crashed or was interrupted, and you want to see the last 10–20 things the agent did.
- You want to audit destructive operations (delete / supersede / deprecate) over the past week.
- A workflow feels slow and you need to see which tool calls happened, in what order.
- Building a paper trail for a Note or Problem after fixing something brittle - paste the relevant entries into the body.

## When NOT to use

- You want totals (call counts, average duration) per tool - use [`forgeplan activity-stats`](/docs/cli/activity-stats/) instead, it aggregates the same data.
- You need a live stream - entries are written per call, but for real-time monitoring run `tail -f .forgeplan/logs/tools-*.jsonl`.
- The workspace is fresh with no history - there is nothing to read yet.

## Usage

```text
forgeplan activity [OPTIONS]
```

## Options

```text
      --since-hours <SINCE_HOURS>  Time window in hours back from now (1..=720, default 24) [default: 24]
      --tool <TOOL>                Filter by tool name. Comma-separated for multiple: "forgeplan_score,forgeplan_activate"
      --status <STATUS>            Filter by status: ok, tool_err, or rpc_err. Omit for all
      --limit <LIMIT>              Cap result set (most recent N). 1..=5000, default 500 [default: 500]
      --json                       Output as JSON for machine consumption
  -h, --help                       Print help
  -V, --version                    Print version
```

## Examples

### Example 1: Last hour of work

```bash
forgeplan activity --since-hours 1
```

Keeps the most recent 500 matching calls by default and prints those retained entries in chronological/log order, not newest first. With this window, entries come from the last 60 minutes. Useful after a session interruption to rebuild context.

### Example 2: Audit destructive ops over the past week

```bash
forgeplan activity --since-hours 168 \
  --tool forgeplan_delete,forgeplan_supersede,forgeplan_deprecate
```

Surfaces every soft-delete / supersede / deprecate over 7 days. Pair with
[`forgeplan undo-last`](/docs/cli/undo-last/) or [`forgeplan restore`](/docs/cli/restore/)
to reverse anything unexpected.

### Example 3: Errors only, machine-readable

```bash
forgeplan activity --status tool_err --limit 50 --json | jq '.entries[]'
```

Pipes JSON output into `jq` for further filtering or for feeding into a script. The `--limit` flag caps the result count so you do not accidentally pull thousands of entries from a long-lived workspace.

## JSON and limits

JSON contains `entries`, `total_scanned`, `returned`, `warnings`, `since_hours`, `_next_action`, and `hints`. Entry fields are `ts`, `tool`, `args_hash`, `duration_ms`, `status`, and `workspace`, with optional `client_info` and `args`. Raw arguments are omitted unless `activity.log_args: true` is enabled. The handler clamps the window to 1–720 hours and limit to 1–5000; an unknown status string simply matches no entries. Empty results are successful reports, not command failures. An initialized workspace is required.

## How it fits the workflow

The activity log is an audit trail of recorded MCP calls, not a complete history of workspace changes. Run [`forgeplan activity-stats`](/docs/cli/activity-stats/) to spot slow or failing tools, then `forgeplan activity --tool <name>` for individual calls. Inspect any unexpected destructive operation before choosing [`forgeplan undo-last`](/docs/cli/undo-last/) or a targeted restore.

## See also

- [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) - MCP equivalent
- [`forgeplan activity-stats`](/docs/cli/activity-stats/) - per-tool aggregates
- [`forgeplan undo-last`](/docs/cli/undo-last/) - reverse the last destructive op
- [`forgeplan restore`](/docs/cli/restore/) - restore a specific soft-deleted artifact
- [CLI overview](/docs/cli/)
