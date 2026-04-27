---
title: forgeplan release
description: "Release an active claim — drop the lock so other sub-agents can pick up the artifact. Idempotent; missing claim is a no-op."
---

`forgeplan release` drops a claim — the artifact returns to the candidate pool, and the next [`forgeplan dispatch`](/docs/cli/dispatch/) can hand it to another agent. It deletes the claim file at `.forgeplan/claims/<id>.yaml`.

By default the command refuses if a different agent holds the claim — you can only release your own work. To override (e.g. after a sub-agent crashed and is no longer running), pass `--force`. Calling release on an artifact with no active claim is a no-op — idempotent — so cleanup scripts can run without checking first.

Mirrors [`forgeplan_release`](/docs/mcp/forgeplan_release/) on the MCP side.

## When to use

- A worker finished its artifact — release so the next dispatch round can give it to someone else.
- A worker crashed or hung — the orchestrator runs `release --force` to free the slot.
- A worker claimed the wrong ID by mistake — release immediately and try again.
- End-of-session cleanup — iterate active claims and release each one before exit.

## When NOT to use

- To delete the artifact itself — release only drops the claim. For the artifact, use [`forgeplan delete`](/docs/cli/delete/).
- To shorten a claim's TTL — release drops the claim entirely. To set a shorter TTL, just call [`forgeplan claim`](/docs/cli/claim/) again with the new value (idempotent for the holder).
- To free a crashed worker's claim without `--force` — the command will refuse, since the agent identity does not match.

## Usage

```text
forgeplan release [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID to release
```

## Options

```text
      --agent <AGENT>  Agent identity. Defaults to `cli/<version>` (or empty when --force)
      --force          Force-release regardless of holder (orchestrator escape hatch)
      --json           Output as JSON for machine consumption
  -h, --help           Print help
  -V, --version        Print version
```

## Examples

### Example 1: Worker releases after finishing

```bash
forgeplan release PRD-057
```

Drops the claim under the default `cli/<version>` identity. Calling again on an already-released artifact is a no-op (no error).

### Example 2: Orchestrator reclaims a crashed sub-agent's slot

```bash
forgeplan release RFC-012 --force
```

The override path when a worker died but its claim has not yet expired. Use this from the orchestrator only — sub-agents should never force-release each other's claims.

### Example 3: Explicit identity for shell-script orchestrators

```bash
forgeplan release SPEC-018 --agent worker-2
```

When a shell script needs to release on behalf of a specific worker, pass `--agent` explicitly. Without `--force`, the agent identity must match the current holder, otherwise the command refuses.

## How it fits the workflow

This closes the multi-agent loop: `dispatch` → `claim` → work → **`release`** → `dispatch` again. After release, the slot returns to the candidate pool, and the next [`forgeplan dispatch`](/docs/cli/dispatch/) call can hand the artifact to a different agent.

## See also

- [`forgeplan_release`](/docs/mcp/forgeplan_release/) — MCP equivalent
- [`forgeplan claim`](/docs/cli/claim/) — acquire the claim
- [`forgeplan claims`](/docs/cli/claims/) — see who holds what
- [`forgeplan dispatch`](/docs/cli/dispatch/) — re-plan after release
