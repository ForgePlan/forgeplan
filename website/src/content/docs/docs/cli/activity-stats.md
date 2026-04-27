---
title: forgeplan activity-stats
description: "Aggregate the activity log by tool name — counts, error rates, p50/p95 duration, total time. Entry point for cost and latency triage."
---

`forgeplan activity-stats` summarises the activity log: one row per tool name, with how many times it ran, how many times it failed, and how long it took (median / 95th percentile / total). It reads the same log files as [`forgeplan activity`](/docs/cli/activity/) but rolls them up so you can answer "where did my session time go?" in a single glance instead of scrolling through hundreds of individual calls.

This is the CLI version of [`forgeplan_activity_stats`](/docs/mcp/forgeplan_activity_stats/) — both produce the same numbers.

A note on percentiles: **p50** is the median (half of calls were faster, half slower); **p95** is the slow tail (95% of calls were faster than this). High p95 with low p50 means most calls are fast but a few are dragging.

## When to use

- Start of a debugging session — find the slowest or most-called tool over the last 24 hours.
- A user reports "it feels slow" and you need a starting point for investigation.
- Pre-release check — compare error counts to last week to spot regressions.
- After a long session, confirm that the destructive-operation count matches what you expected.

## When NOT to use

- You need to see individual calls (timestamps, arguments) — use [`forgeplan activity`](/docs/cli/activity/) instead, it does not aggregate.
- You want a methodology gate (block work until something passes) — stats are observability, not validation.

## Usage

```text
forgeplan activity-stats [OPTIONS]
```

## Options

```text
      --since-hours <SINCE_HOURS>  Time window in hours (1..=720, default 24) [default: 24]
      --json                       Output as JSON for machine consumption
  -h, --help                       Print help
  -V, --version                    Print version
```

## Examples

### Example 1: Default 24-hour summary

```bash
forgeplan activity-stats
```

Prints rows ordered by total time spent (highest first) — the most time-consuming tool appears at the top. Quick answer to "where did my session time go?".

### Example 2: Full-month view

```bash
forgeplan activity-stats --since-hours 720
```

Use this for monthly retrospectives or when investigating a slow workflow that may have started days ago. `720` hours (30 days) is the maximum window allowed.

### Example 3: Find slow tools (p95 > 1 second)

```bash
forgeplan activity-stats --json | jq '.stats[] | select(.p95_ms > 1000)'
```

Filters to tools where the slow-tail (p95) duration exceeds 1000 ms (1 second). A typical first cut when investigating "the agent feels slow today" — these are the tools dragging the experience down.

## How it fits the workflow

Pair with [`forgeplan activity`](/docs/cli/activity/) for a two-step investigation: run `activity-stats` first to find the slow or failing tool, then run `activity --tool <name>` to see the individual calls. If errors are the issue, narrow further with `--status tool_err`.

## See also

- [`forgeplan_activity_stats`](/docs/mcp/forgeplan_activity_stats/) — MCP equivalent
- [`forgeplan activity`](/docs/cli/activity/) — entry-level drill-down
- [`forgeplan undo-last`](/docs/cli/undo-last/) — pair with stats to reverse misfires
- [CLI overview](/docs/cli/)
