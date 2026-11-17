---
title: forgeplan claim
description: "Claim an artifact for exclusive work - TTL-based advisory lock for multi-agent dispatch (PRD-057)."
---

`forgeplan claim` reserves an artifact for a specific agent, so other agents know not to touch it. It writes a small file at `.forgeplan/claims/<id>.yaml` containing the agent identity and an expiry time (TTL - time-to-live, after which the claim is considered abandoned and another agent can pick up the work). The expiry defaults to 30 minutes and caps at 24 hours.

This is an **advisory lock**, not a hard lock - no other Forgeplan command refuses to run because something is claimed. The convention is that agents check [`forgeplan claims`](/docs/cli/claims/) before starting work and respect what they see. The dispatcher ([`forgeplan dispatch`](/docs/cli/dispatch/)) does honour claims and excludes already-claimed artifacts from its plans.

Two coordination behaviors:

- **Live-holder check** — a different holder observed by the read is rejected. This is a check-then-write advisory protocol, not an atomic cross-process compare-and-set; simultaneous CLI claims are not a guaranteed mutex.
- **Renewal** — the same identity can replace its claim with a refreshed expiry. Use a distinct `--agent` per worker and respect live claims.

Mirrors [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) on the MCP side.

## When to use

- A sub-agent received a bucket from [`forgeplan dispatch`](/docs/cli/dispatch/) - claim the artifact before editing any files.
- Long-running work (large refactor, multi-PR feature) - call `claim` again every 20 minutes or so to refresh the expiry before it runs out.
- An orchestrator script wants to claim on behalf of a sub-agent that does not speak MCP - pass `--agent worker-1` explicitly.

## When NOT to use

- Single-agent workflow - nothing to race against, no need for a claim.
- You expect a hard lock that blocks other commands - claims are advisory; agents must voluntarily check them.
- Without thinking about TTL - the default 30 minutes prevents abandoned claims from blocking the workspace forever. Set it higher only if you plan to renew before it expires.

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

Reserves `PRD-057` for 30 minutes under `cli-<crate-version>` (a hyphen, not a slash). The help block above still describes the older slash form; the implementation rejects `/` in an explicit CLI agent identity. Choose a distinct `--agent` for each worker: workers sharing the default identity can renew one another's claim.

### Example 2: Orchestrator claims on behalf of a worker

```bash
forgeplan claim RFC-012 --agent worker-2 --ttl-minutes 60
```

Useful when the orchestrator is a shell script driving sub-agents that do not speak MCP directly. The 60-minute TTL gives `worker-2` a wider window before it needs to renew.

### Example 3: Renew an existing claim

```bash
forgeplan claim PRD-057 --ttl-minutes 30
```

The same agent refreshes both `claimed_at` and `expires_at`; the supplied note replaces the previous note (omitting `--note` clears it). Use the same explicit `--agent` when renewing a worker's claim.

With `--json`, success contains `id`, `agent_id`, `claimed_at`, `expires_at`,
`note` (possibly null), `_next_action`, and `hints`. A live-holder conflict
exits `1` and returns `error: "already_held"`, `id`, `agent_id`, `expires_at`,
`_next_action`, and `hints`. Other validation/I/O errors are ordinary CLI
errors, not that JSON envelope. TTL must be 1–1440 minutes. An explicit agent
identity is trimmed and limited to 64 bytes; path/control/format characters
are rejected. A syntactically valid forward-reference ID can be claimed even
before the artifact exists.

## How it fits the workflow

This is step 2 of the multi-agent loop: `dispatch` → **`claim`** → work → `release`. After [`forgeplan dispatch`](/docs/cli/dispatch/) hands out buckets, each sub-agent claims its artifact before touching files. When work is done (or the agent crashes), the slot is freed via [`forgeplan release`](/docs/cli/release/) - use `--force` on `release` when reaping a crashed worker's claim.

## See also

- [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) - MCP equivalent
- [`forgeplan release`](/docs/cli/release/) - drop an active claim
- [`forgeplan claims`](/docs/cli/claims/) - list active claims
- [`forgeplan dispatch`](/docs/cli/dispatch/) - produces the work plan claims protect
