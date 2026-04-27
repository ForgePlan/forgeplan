---
title: forgeplan_release
description: "Release an active claim — drop the lock so other sub-agents can pick up the artifact."
---

Removes the claim file at `.forgeplan/claims/<id>.yaml`. By default the call refuses
when a different agent holds the claim — pass `force: true` (the orchestrator's escape
hatch) to override after a sub-agent crash. Missing claim is a no-op (idempotent).
Holds the workspace lock for the duration of the write so concurrent claim/release
calls cannot interleave.

**Category**: Multi-agent

## When an agent calls it

- Worker finishes the artifact and frees the slot for the next dispatch round.
- Worker crashes / times out — orchestrator force-releases with `agent: null, force: true`.
- Mistaken claim: agent grabbed the wrong ID, releases immediately to retry.
- Cleanup at session end: walk active claims and release each one before exit.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID whose claim to release. |
| `agent` | `string` | no | Agent identity (must match the holder unless `force: true`). Defaults to the MCP caller's `clientInfo`. |
| `force` | `bool` | no (default `false`) | Force-release regardless of holder — orchestrator override for crashed sub-agents. |

_Schema source: `crates/forgeplan-mcp/src/types.rs::ReleaseParams`_

## Returns

```json
{
  "id": "PRD-057",
  "released": true,
  "force": false,
  "_next_action": "Released claim on `PRD-057`."
}
```

Failure when not the holder and not forcing:

```json
{
  "ok": false,
  "error": "claim held by worker-2, not you",
  "_next_action": "Use `force: true` (orchestrator override) if the holder has crashed."
}
```

## Example invocation

Worker releasing after work:

```json
{ "id": "PRD-057" }
```

Orchestrator reaping a crashed sub-agent:

```json
{ "id": "RFC-012", "force": true }
```

Explicit identity for shell-driven orchestrators:

```json
{ "id": "SPEC-018", "agent": "worker-2" }
```

## Typical sequence

1. [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) → buckets per agent.
2. [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) → worker locks its bucket head.
3. Worker does the artifact / code work.
4. `forgeplan_release` → free the slot.
5. Orchestrator re-dispatches.

## CLI equivalent

[`forgeplan release <id>`](/docs/cli/) — same semantics.

## See also

- [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) — acquire the claim
- [`forgeplan_claims`](/docs/mcp/forgeplan_claims/) — see who holds what
- [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) — re-plan after release
