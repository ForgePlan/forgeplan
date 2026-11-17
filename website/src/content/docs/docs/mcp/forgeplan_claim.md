---
title: forgeplan_claim
description: "Claim an artifact for exclusive work. Writes .forgeplan/claims/<id>.yaml with the caller's agent identity, timestamp, and TTL. Fails if a live claim by a different agent exists - same-agent calls renew the TTL. Advisory: other tools do not block on claims, but orchestrators should use `forgeplan_claims --active` to avoid double-assigning work."
---

Writes `.forgeplan/claims/<id>.yaml` to record an agent's intent to work on an artifact. A workspace write lock serializes claim/release calls, but the persisted claim is advisory: it does not lock source files or block other tools. A live claim held by a different agent is refused. Calling again as the same agent rewrites the timestamps, TTL and note; this is renewal, not a no-write idempotent operation. Consult [`forgeplan_claims`](/docs/mcp/forgeplan_claims/) before dispatching work.

**Category**: Multi-agent

## When an agent calls it

- A sub-agent picks an artifact from a `forgeplan_dispatch` bucket and claims it before touching files.
- Long-running work (R3-grade refactor, multi-PR feature): renew the claim with the same
  call before the TTL expires.
- An orchestrator claims on behalf of a worker by passing `agent: "worker-1"` explicitly.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `agent` | `string \| null` | no | Agent identity ("name/version" or free-form). Optional - defaults to the MCP caller's clientInfo when omitted. Providing this explicitly lets an orchestrator claim on behalf of a sub-agent. |
| `id` | `string` | yes | Artifact ID to claim (e.g. `PRD-057`). Normalized to uppercase on disk. |
| `note` | `string \| null` | no | Optional free-form note surfaced by `forgeplan_claims --active`. |
| `ttl_minutes` | `integer \| null` | no | Time-to-live in minutes. Default 30, max 1440 (24h), min 1. `format`: `uint32`. `minimum`: `0`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The payload contains `id` (uppercased), `agent_id`, `claimed_at`, `expires_at`, an optional `note`, and `_next_action`. `note` is omitted when absent. This handler does not add workspace trace fields.

A collision is an MCP error result with text identifying the current holder and expiry, plus an advisory hint to select other work. It is not a structured `ok: false` object, and it does not authorize forcing another agent's claim away.

## Guards and side effects

- Only `id` is schema-required. The handler uses a non-blank explicit `agent` first, then the MCP `clientInfo` identity. Without either it refuses; do not make the optional JSON field universally required in clients.
- TTL defaults to **30 minutes**; values below 1 or above 1440 are refused. The schema's unsigned-integer minimum of 0 is not the runtime lower bound.
- IDs must be non-empty ASCII ID tokens starting with a letter; stored claim filenames and IDs are uppercased. Agent identity is trimmed, bounded to 64 bytes, and rejects control/spoofing characters. Holder identity comparisons are exact.
- An expired claim can be overwritten. Same-holder renewal replaces the note too, so omitting it does not preserve the old note.
- The claim store does not verify that the artifact exists in LanceDB. A successful claim proves only that the coordination file was written, not artifact existence, authorization or completed work.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "RFC-001",
  "agent": "worker-1",
  "ttl_minutes": 30
}
```

## Typical sequence

1. [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) - orchestrator builds buckets.
2. Worker calls `forgeplan_claim` on its assigned artifact.
3. Worker does the actual code / artifact edits.
4. [`forgeplan_release`](/docs/mcp/forgeplan_release/) - drop the claim when done.

## CLI equivalent

[`forgeplan claim`](/docs/cli/) - same semantics, used by orchestrators that drive
sub-agents through shell rather than MCP.

## See also

- [`forgeplan_release`](/docs/mcp/forgeplan_release/) - drop an active claim
- [`forgeplan_claims`](/docs/mcp/forgeplan_claims/) - list active claims
- [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) - produces the work plan claims protect (full PRD-057 protocol)
