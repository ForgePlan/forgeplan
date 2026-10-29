---
title: forgeplan_claims
description: "List live claims in the workspace - who is working on what right now."
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
- Health checks: any non-zero `skipped` count signals corrupt claim files worth investigating.
- Session-start protocol after a crash: list orphan claims, force-release the dead ones.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `active` | `bool` | no (default `false`) | Reserved for future filters; currently always returns only live claims. |

_Schema source: `crates/forgeplan-mcp/src/types.rs::ClaimsListParams`_

## Returns
Illustrative response; a missing optional `note` is omitted:


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

`skipped > 0` means at least one claim file failed to parse or exceeded the size cap.
The counter does not identify the file; inspect `.forgeplan/claims/` to locate it.

## Example invocation

```json
{}
```

(`active` defaults to `false` - the field is reserved; you do not need to pass it.)

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

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

List live claims in the workspace, sorted by expiry ascending. Skips expired claims (they're considered practically released). Used by orchestrators to build dispatch plans and by sub-agents to avoid double-claiming.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `active` | no | boolean | Reserved for future filters; currently always returns only live claims. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "active": {
      "default": false,
      "description": "Reserved for future filters; currently always returns only live claims.",
      "type": "boolean"
    },
    "workspace": {
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "ClaimsListParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
