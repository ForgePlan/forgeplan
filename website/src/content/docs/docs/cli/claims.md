---
title: forgeplan claims
description: "List live claims in the workspace — who is working on what right now. Sorted by expiry, soonest first."
---

`forgeplan claims` lists every active claim in the workspace — which agent is currently working on which artifact, and how soon their claim expires. Claims are sorted by time-to-expiry, soonest first, so the most urgent slots appear at the top. Expired claims are skipped (treated as released).

Read-only and lock-free: an orchestrator can poll this once per second without slowing down agents that are writing claims. If a claim file is corrupt or oversized, it is silently skipped and counted under `skipped` in the output — `forgeplan health` can then surface it for cleanup.

Mirrors [`forgeplan_claims`](/docs/mcp/forgeplan_claims/) on the MCP side.

## When to use

- Orchestrator monitoring on each dispatch round — "what work is currently in progress?"
- A sub-agent before claiming — "did someone else beat me to this artifact?"
- Health check — a non-zero `skipped` count signals corrupt claim files worth investigating.
- Recovery after a crash — list claims that no live agent is holding, then force-release the dead ones with [`forgeplan release --force`](/docs/cli/release/).

## When NOT to use

- You need lifecycle state (`draft` / `active` / `superseded`) — that is separate from claims; use [`forgeplan list`](/docs/cli/list/) or `forgeplan show <id>`.
- You want to modify state — this command only reads. Use [`forgeplan claim`](/docs/cli/claim/) to acquire and [`forgeplan release`](/docs/cli/release/) to drop.
- You need long-term audit history — claims are temporary (max 24-hour TTL). For historical records, query [`forgeplan activity`](/docs/cli/activity/).

## Usage

```text
forgeplan claims [OPTIONS]
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: Default text-mode listing

```bash
forgeplan claims
```

Prints a table of live claims sorted by expiry. Typical output:

```text
ID        agent      expires_in   note
PRD-057   worker-1   12m          implementing FR-003
RFC-012   worker-2   58m          —
```

### Example 2: JSON output for orchestrator polling

```bash
forgeplan claims --json | jq '.claims[] | select(.agent_id == "worker-1")'
```

Filters the active claims down to a specific worker. Useful in a dispatcher script that needs to decide whether to redispatch new work to that worker.

### Example 3: Detect corrupt claim files

```bash
forgeplan claims --json | jq '.skipped'
```

A non-zero `skipped` value means at least one claim file failed to parse or exceeded the maximum size — these used to be dropped silently, now they are surfaced explicitly. Run [`forgeplan health`](/docs/cli/health/) next to identify which file is broken.

## How it fits the workflow

This is the monitoring layer of the multi-agent loop. Between `dispatch` rounds, the orchestrator calls `claims` to see live work; each sub-agent calls it before its own `claim` to avoid collisions. Pair with [`forgeplan dispatch`](/docs/cli/dispatch/) — the dispatcher reads claims internally and excludes already-claimed artifacts from the plan.

## See also

- [`forgeplan_claims`](/docs/mcp/forgeplan_claims/) — MCP equivalent
- [`forgeplan claim`](/docs/cli/claim/) — acquire a claim
- [`forgeplan release`](/docs/cli/release/) — drop a claim
- [`forgeplan dispatch`](/docs/cli/dispatch/) — multi-agent work plan
