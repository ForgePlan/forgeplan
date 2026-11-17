---
title: forgeplan_release
description: "Release an active claim. Refuses if the claim is held by a different agent (use `force: true` to override - the orchestrator's escape hatch for a crashed sub-agent). Missing claim is a no-op (idempotent)."
---

Removes `.forgeplan/claims/<id>.yaml`, not an artifact, source-file lock or Forgeplan process. By default a **live** claim held by another agent is refused; an expired claim can be removed. `force: true` is an explicit orchestrator override for a confirmed abandoned claim, not an automatic recovery step. A missing claim is a no-op only after identity/ID validation. The workspace lock serializes concurrent claim/release calls.

**Category**: Multi-agent

## When an agent calls it

- Worker finishes the artifact and frees the slot for the next dispatch round.
- Worker crashes / times out - orchestrator force-releases with `agent: null, force: true`.
- Mistaken claim: agent grabbed the wrong ID, releases immediately to retry.
- Cleanup at session end: walk active claims and release each one before exit.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `agent` | `string \| null` | no | Required unless `force=true`. |
| `force` | `boolean` | no | Force-release regardless of holder (orchestrator escape hatch). Schema default: `false`. |
| `id` | `string` | yes | - |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The success payload contains `id`, `released: true`, `force`, `resolved_workspace`, `resolved_via` and `_next_action`. `released: true` also covers a missing claim; it does not prove that a file existed.

Another live holder produces MCP error text such as `claim held by worker-2, not you`, with an advisory hint. There is no `ok: false` / `error` JSON object.

## Guards and side effects

- `force` defaults to `false`. A non-blank explicit `agent` takes precedence over the MCP handshake identity; with neither identity the non-forced path fails even if the claim does not exist.
- A forced call may omit `agent` and use no handshake identity. This removes the coordination file regardless of holder; the tool has no interactive confirmation, so obtain authorization before overriding.
- Non-forced identity validation and ID-token validation still apply. The agent string is trimmed; a live holder must match exactly. Expiry removes the holder restriction, not the need for a valid caller identity.
- This only releases an advisory claim. It does not stop a running worker, undo its edits or certify that the artifact's work is finished.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "RFC-001",
  "agent": "worker-1"
}
```

## Typical sequence

1. [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) → buckets per agent.
2. [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) → worker locks its bucket head.
3. Worker does the artifact / code work.
4. `forgeplan_release` → free the slot.
5. Orchestrator re-dispatches.

## CLI equivalent

[`forgeplan release <id>`](/docs/cli/) - same semantics.

## See also

- [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) - acquire the claim
- [`forgeplan_claims`](/docs/mcp/forgeplan_claims/) - see who holds what
- [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) - re-plan after release
