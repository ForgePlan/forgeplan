---
title: forgeplan_claim
description: "Claim an artifact for exclusive work — TTL-based advisory lock for multi-agent dispatch."
---

Writes `.forgeplan/claims/<id>.yaml` declaring that a specific agent is working on the
artifact. Holds the workspace lock for the duration of the write so two sub-agents cannot
race the same claim. Fails with a clear error when a different agent already holds a live
claim; same-agent calls renew the TTL (idempotent for the holder). Advisory by design —
no other tool blocks on claims, but orchestrators should consult
[`forgeplan_claims`](/docs/mcp/forgeplan_claims/) before dispatching parallel work.

**Category**: Multi-agent

## When an agent calls it

- A sub-agent picks an artifact from a `forgeplan_dispatch` bucket and claims it before touching files.
- Long-running work (R3-grade refactor, multi-PR feature): renew the claim with the same
  call before the TTL expires.
- An orchestrator claims on behalf of a worker by passing `agent: "worker-1"` explicitly.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to claim (e.g. `PRD-057`). Normalized to uppercase on disk. |
| `agent` | `string` | no | Agent identity (`"name/version"` or free-form). Defaults to the MCP caller's `clientInfo` when omitted. |
| `ttl_minutes` | `number` | no (default 30, max 1440) | TTL in minutes. Hard cap is 24 h to prevent zombie claims. |
| `note` | `string` | no | Free-form note surfaced by `forgeplan_claims`. |

_Schema source: `crates/forgeplan-mcp/src/types.rs::ClaimParams`_

## Returns

```json
{
  "id": "PRD-057",
  "agent_id": "worker-1",
  "claimed_at": "2026-04-26T10:00:00Z",
  "expires_at": "2026-04-26T10:30:00Z",
  "note": "implementing FR-003",
  "_next_action": "Claimed `PRD-057` for `worker-1`. Release with `forgeplan_release PRD-057` ..."
}
```

On collision the response is an error with `_next_action` advising "either work on a
different artifact, wait for TTL expiry, or ask the orchestrator to force-release".

## Example invocation

Worker claiming with default TTL:

```json
{ "id": "PRD-057", "note": "implementing FR-003" }
```

Orchestrator claiming on a worker's behalf:

```json
{ "id": "RFC-012", "agent": "worker-2", "ttl_minutes": 60 }
```

Renewing an existing claim (same agent, any TTL):

```json
{ "id": "PRD-057", "ttl_minutes": 30 }
```

## Typical sequence

1. [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) — orchestrator builds buckets.
2. Worker calls `forgeplan_claim` on its assigned artifact.
3. Worker does the actual code / artifact edits.
4. [`forgeplan_release`](/docs/mcp/forgeplan_release/) — drop the claim when done.

## CLI equivalent

[`forgeplan claim`](/docs/cli/) — same semantics, used by orchestrators that drive
sub-agents through shell rather than MCP.

## See also

- [`forgeplan_release`](/docs/mcp/forgeplan_release/) — drop an active claim
- [`forgeplan_claims`](/docs/mcp/forgeplan_claims/) — list active claims
- [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) — produces the work plan claims protect (full PRD-057 protocol)
