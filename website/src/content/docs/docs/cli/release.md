---
title: forgeplan release
description: "Release an advisory artifact claim. Missing claims are successful no-ops; a different live holder requires an explicit force override. This is not an artifact write lock."
---

`forgeplan release` drops a claim - the artifact returns to the candidate pool, and the next [`forgeplan dispatch`](/docs/cli/dispatch/) can hand it to another agent. It deletes the claim file at `.forgeplan/claims/<id>.yaml`.

By default the command refuses if a different agent holds the claim. `--agent` is a caller-supplied coordination label, not authenticated ownership. To override after confirming a worker is no longer using the artifact, an orchestrator can pass `--force`. A missing claim is a successful no-op. Claims are advisory records, not artifact write locks; release does not stop a running worker or alter lifecycle status.

Mirrors [`forgeplan_release`](/docs/mcp/forgeplan_release/) on the MCP side.

## When to use

- A worker finished its artifact - release so the next dispatch round can give it to someone else.
- A worker crashed or hung - the orchestrator runs `release --force` to free the slot.
- A worker claimed the wrong ID by mistake - release immediately and try again.
- End-of-session cleanup - iterate active claims and release each one before exit.

## When NOT to use

- To delete the artifact itself - release only drops the claim. For the artifact, use [`forgeplan delete`](/docs/cli/delete/).
- To shorten a claim's TTL - release drops the claim entirely. To set a shorter TTL, just call [`forgeplan claim`](/docs/cli/claim/) again with the new value (idempotent for the holder).
- To free a crashed worker's claim without `--force` - the command will refuse, since the agent identity does not match.

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
# claimed as: forgeplan claim PRD-057 --agent worker-1 --ttl-minutes 60
forgeplan release PRD-057 --agent worker-1
```

**Release under the same identity you claimed with.** `release` does not inherit it; with no `--agent` it identifies as `cli/<version>`, which will not match a claim made as `worker-1`, and the command refuses.

If you claimed with no `--agent` at all, then plain `forgeplan release PRD-057` is correct: both sides use the same default identity. Calling again on an already-released artifact is a no-op (no error).

When the identities do not match, the fix is to name the holder. **Not** `--force`, which drops the claim regardless of who owns it (PROB-095).

### Example 2: Orchestrator reclaims a crashed sub-agent's slot

```bash
forgeplan release RFC-012 --force
```

The override path when a worker died but its claim has not yet expired. Use this from the orchestrator only - sub-agents should never force-release each other's claims.

### Example 3: Explicit identity for shell-script orchestrators

```bash
forgeplan release SPEC-018 --agent worker-2
```

When a shell script needs to release on behalf of a specific worker, pass `--agent` explicitly. Without `--force`, the agent identity must match the current holder, otherwise the command refuses.

## JSON and errors

Success emits `id`, `released: true`, `force`, and `_next_action`; `released: true` also covers a missing claim. A holder mismatch exits 1 and emits `error: "not_held_by_requester"`, `id`, `held_by`, `_next_action`, and `_alternative_action` in JSON. Other failures, including missing workspace or claim-store errors, follow the ordinary CLI error path, not this JSON shape.

## How it fits the workflow

This closes the multi-agent loop: `dispatch` → `claim` → work → **`release`** → `dispatch` again. After release, the slot returns to the candidate pool, and the next [`forgeplan dispatch`](/docs/cli/dispatch/) call can hand the artifact to a different agent.

## See also

- [`forgeplan_release`](/docs/mcp/forgeplan_release/) - MCP equivalent
- [`forgeplan claim`](/docs/cli/claim/) - acquire the claim
- [`forgeplan claims`](/docs/cli/claims/) - see who holds what
- [`forgeplan dispatch`](/docs/cli/dispatch/) - re-plan after release
