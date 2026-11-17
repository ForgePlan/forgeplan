---
title: forgeplan_claims
description: "List live claims in the workspace, sorted by expiry ascending. Skips expired claims (they're considered practically released). Used by orchestrators to build dispatch plans and by sub-agents to avoid double-claiming."
---

Returns every non-expired claim in `.forgeplan/claims/`, sorted by expiry ascending
(most-urgent first). Skips claims past their TTL - they are considered practically
released. Read-only and lock-free by design (audit-driven): an orchestrator polling at
1 Hz must not serialize sub-agent writes. Malformed claim files are skipped with a
counter so health checks can surface them.

**Category**: Multi-agent

## When an agent calls it

- Orchestrator at every dispatch tick: "what work is already in flight?".
- Sub-agent before claiming: "did another worker beat me to this artifact?".
- Health checks: a non-zero `skipped` count identifies files worth investigating, including parse, size or concurrent-read problems.
- After a crash: inspect remaining live claims and confirm the owner is no longer working before deliberately force-releasing one.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `active` | `boolean` | no | Reserved for future filters; currently always returns only live claims. Schema default: `false`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON inside MCP text content. Values below are illustrative.

```json
{
  "count": 2,
  "skipped": 0,
  "claims": [
    {
      "id": "PRD-057",
      "agent_id": "worker-1",
      "claimed_at": "2026-04-26T10:00:00Z",
      "expires_at": "2026-04-26T10:30:00Z",
      "note": "implementing FR-003"
    },
    {
      "id": "RFC-012",
      "agent_id": "worker-2",
      "claimed_at": "2026-04-26T10:05:00Z",
      "expires_at": "2026-04-26T11:05:00Z"
    }
  ],
  "_next_action": "2 active claim(s). Plan around them: `forgeplan_dispatch agents=3`."
}
```

`skipped > 0` means at least one claim file failed to parse or exceeded the size cap;
a partial-read race may also cause a skipped file. Inspect `.forgeplan/claims/`
and the health report rather than assuming every skipped file is corrupt.
`note` is omitted when absent, not serialized as `null`. `active` is reserved and
does not change the live-only filter. Claims are advisory coordination records,
not write locks or proof that the named agent is currently running.
Workspace-resolution and claim-list failures return tool errors, not a zero-count success.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. `forgeplan_claims` - see who is busy.
2. [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) - plan around the live claims.
3. Hand each bucket to a sub-agent that calls [`forgeplan_claim`](/docs/mcp/forgeplan_claim/).

## CLI equivalent

[`forgeplan claims`](/docs/cli/) - same data; orchestrators that drive workers via shell
poll this command.

## See also

- [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) - acquire a claim
- [`forgeplan_release`](/docs/mcp/forgeplan_release/) - drop a claim
- [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) - multi-agent work plan (full PRD-057 protocol)
