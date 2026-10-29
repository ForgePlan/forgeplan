---
title: forgeplan activity
description: "Inspect recorded MCP calls, statuses, arguments and durations from the activity journal."
---

`forgeplan activity` reads recorded MCP tool invocations from `.forgeplan/logs/tools-YYYY-MM-DD.jsonl`: tool name, arguments, status (`ok`, `tool_err`, or `rpc_err`), and duration. MCP logging is best-effort, so this is not a complete audit of every CLI command or every workspace change.

This is the CLI version of [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) (the MCP tool); both read the same log files.

## When to use

- A session crashed or was interrupted, and you want to see the last 10–20 things the agent did.
- You want to audit destructive operations (delete / supersede / deprecate) over the past week.
- A workflow feels slow and you need to see which tool calls happened, in what order.
- Building a paper trail for a Note or Problem after fixing something brittle - paste the relevant entries into the body.

## When NOT to use

- You want per-tool call counts, p50/p95 duration, and total duration - use [`forgeplan activity-stats`](/docs/cli/activity-stats/), which aggregates the same recorded data.
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

Prints matching recorded calls from the last 60 minutes, subject to the default 500-entry limit. Useful after a session interruption to rebuild context.

### Example 2: Audit destructive ops over the past week

```bash
forgeplan activity --since-hours 168 \
  --tool forgeplan_delete,forgeplan_supersede,forgeplan_deprecate
```

Shows matching recorded operations, subject to the result limit. Inspect the operation and restore receipt before choosing [`undo-last`](/docs/cli/undo-last/) or [`restore`](/docs/cli/restore/); this is not an exhaustive seven-day audit.

### Example 3: Errors only, machine-readable

```bash
forgeplan activity --status tool_err --limit 50 --json | jq '.entries[]'
```

Pipes JSON output into `jq` for further filtering or for feeding into a script. The `--limit` flag caps the result count so you do not accidentally pull thousands of entries from a long-lived workspace.

## How it fits the workflow

Use [`forgeplan activity-stats`](/docs/cli/activity-stats/) to spot slow or failing MCP tools, then `forgeplan activity --tool <name>` to inspect individual recorded calls. If an unexpected destructive operation appears, inspect it and its restore receipt before using [`forgeplan undo-last`](/docs/cli/undo-last/).

## See also

- [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) - MCP equivalent
- [`forgeplan activity-stats`](/docs/cli/activity-stats/) - per-tool aggregates
- [`forgeplan undo-last`](/docs/cli/undo-last/) - reverse the last destructive op
- [`forgeplan restore`](/docs/cli/restore/) - restore a specific soft-deleted artifact
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan activity --help`.

```text
Query the activity log — append-only JSONL record of every MCP tool invocation at .forgeplan/logs/tools-YYYY-MM-DD.jsonl. Use this to reconstruct what the agent did over a time window, attribute LLM-token spend, or audit destructive operations

Usage: forgeplan activity [OPTIONS]

Options:
      --since-hours <SINCE_HOURS>  Time window in hours back from now (1..=720, default 24) [default: 24]
      --tool <TOOL>                Filter by tool name. Comma-separated for multiple: "forgeplan_score,forgeplan_activate"
      --status <STATUS>            Filter by status: ok, tool_err, or rpc_err. Omit for all
      --limit <LIMIT>              Cap result set (most recent N). 1..=5000, default 500 [default: 500]
      --json                       Output as JSON for machine consumption
  -h, --help                       Print help
  -V, --version                    Print version
```
<!-- forgeplan-runtime:end -->
