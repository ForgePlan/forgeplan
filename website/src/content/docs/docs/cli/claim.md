---
title: forgeplan claim
description: "Claim an artifact for exclusive work — TTL-based advisory lock for multi-agent dispatch (PRD-057)."
---

`forgeplan claim` reserves an artifact for a specific agent, so other agents know not to touch it. It writes a small file at `.forgeplan/claims/<id>.yaml` containing the agent identity and an expiry time (TTL — time-to-live, after which the claim is considered abandoned and another agent can pick up the work). The expiry defaults to 30 minutes and caps at 24 hours.

This is an **advisory lock**, not a hard lock — no other Forgeplan command refuses to run because something is claimed. The convention is that agents check [`forgeplan claims`](/docs/cli/claims/) before starting work and respect what they see. The dispatcher ([`forgeplan dispatch`](/docs/cli/dispatch/)) does honour claims and excludes already-claimed artifacts from its plans.

Two safety properties:

- **No double-claim** — if a different agent already holds a live claim for this artifact, the call fails with a clear error message.
- **Renewal is idempotent** — the same agent calling `claim` again refreshes the expiry without changing the holder. Use this on long-running work to keep the claim alive.

Mirrors [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) on the MCP side.

## When to use

- A sub-agent received a bucket from [`forgeplan dispatch`](/docs/cli/dispatch/) — claim the artifact before editing any files.
- Long-running work (large refactor, multi-PR feature) — call `claim` again every 20 minutes or so to refresh the expiry before it runs out.
- An orchestrator script wants to claim on behalf of a sub-agent that does not speak MCP — pass `--agent worker-1` explicitly.

## When NOT to use

- Single-agent workflow — nothing to race against, no need for a claim.
- You expect a hard lock that blocks other commands — claims are advisory; agents must voluntarily check them.
- Without thinking about TTL — the default 30 minutes prevents abandoned claims from blocking the workspace forever. Set it higher only if you plan to renew before it expires.

## Usage

```text
forgeplan claim [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID to claim (e.g. PRD-057)
```

## Options

```text
      --agent <AGENT>              Agent identity ("name/version"). Defaults to `cli/<version>`
      --ttl-minutes <TTL_MINUTES>  Time-to-live in minutes (default 30, max 1440 = 24h, min 1) [default: 30]
      --note <NOTE>                Optional free-form note surfaced by `forgeplan claims`
      --json                       Output as JSON for machine consumption
  -h, --help                       Print help
  -V, --version                    Print version
```

## Examples

### Example 1: Sub-agent claims with default TTL

```bash
forgeplan claim PRD-057 --note "implementing FR-003"
```

Reserves `PRD-057` for 30 minutes under the default `cli/<version>` identity. The note appears in [`forgeplan claims`](/docs/cli/claims/) so the orchestrator can see what each agent is doing.

### Example 2: Orchestrator claims on behalf of a worker

```bash
forgeplan claim RFC-012 --agent worker-2 --ttl-minutes 60
```

Useful when the orchestrator is a shell script driving sub-agents that do not speak MCP directly. The 60-minute TTL gives `worker-2` a wider window before it needs to renew.

### Example 3: Renew an existing claim

```bash
forgeplan claim PRD-057 --ttl-minutes 30
```

The same agent calling `claim` again on its own artifact extends the expiry without changing anything else. Use this on long-running refactors before the TTL runs out.

## How it fits the workflow

This is step 2 of the multi-agent loop: `dispatch` → **`claim`** → work → `release`. After [`forgeplan dispatch`](/docs/cli/dispatch/) hands out buckets, each sub-agent claims its artifact before touching files. When work is done (or the agent crashes), the slot is freed via [`forgeplan release`](/docs/cli/release/) — use `--force` on `release` when reaping a crashed worker's claim.

## See also

- [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) — MCP equivalent
- [`forgeplan release`](/docs/cli/release/) — drop an active claim
- [`forgeplan claims`](/docs/cli/claims/) — list active claims
- [`forgeplan dispatch`](/docs/cli/dispatch/) — produces the work plan claims protect
