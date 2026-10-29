---
title: forgeplan_claim
description: "Claim an artifact for exclusive work - TTL-based advisory lock for multi-agent dispatch."
---

Writes `.forgeplan/claims/<id>.yaml` declaring that a specific agent is working on the
artifact. Holds the workspace lock for the duration of the write so two sub-agents cannot
race the same claim. Fails with a clear error when a different agent already holds a live
claim; same-agent calls renew the TTL and update its timestamps. Advisory by design -
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
| `agent` | `string` | no | Agent identity (`"name/version"` or free-form), at most 64 UTF-8 bytes after trimming. Defaults to the MCP caller's `clientInfo`; without either identity the call fails. Control/format characters are rejected. |
| `ttl_minutes` | `integer` | no (default 30) | `1..=1440` minutes; values outside this range are rejected. Renew for longer work. |
| `note` | `string` | no | Free-form note surfaced by `forgeplan_claims`. |

_Schema source: `crates/forgeplan-mcp/src/types.rs::ClaimParams`_

## Returns
Illustrative response:


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

On collision the handler returns an error and recommends claiming a different artifact
through `forgeplan_dispatch`. Do not force-release another worker merely to bypass it.

## Example invocation

Worker claiming with default TTL:

```json
{ "id": "PRD-057", "note": "implementing FR-003" }
```

Orchestrator claiming on a worker's behalf:

```json
{ "id": "RFC-012", "agent": "worker-2", "ttl_minutes": 60 }
```

Renewing an existing claim (same agent, any TTL in the valid range):

```json
{ "id": "PRD-057", "ttl_minutes": 30 }
```

## Typical sequence

1. [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) - orchestrator builds buckets.
2. Worker calls `forgeplan_claim` on its assigned artifact.
3. Worker does the actual code / artifact edits.
4. [`forgeplan_release`](/docs/mcp/forgeplan_release/) - drop the claim when done.

## CLI equivalent

[`forgeplan claim`](/docs/cli/) uses the same advisory claim store, but defaults to the
CLI identity, not MCP `clientInfo`. Explicit CLI `--agent` values reject `/`;
use a value such as `worker-1` and pass the matching holder when releasing.

## See also

- [`forgeplan_release`](/docs/mcp/forgeplan_release/) - drop an active claim
- [`forgeplan_claims`](/docs/mcp/forgeplan_claims/) - list active claims
- [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) - produces the work plan claims protect (full PRD-057 protocol)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Claim an artifact for exclusive work. Writes .forgeplan/claims/<id>.yaml with the caller's agent identity, timestamp, and TTL. Fails if a live claim by a different agent exists — same-agent calls renew the TTL. Advisory: other tools do not block on claims, but orchestrators should use `forgeplan_claims --active` to avoid double-assigning work.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `agent` | no | string \| null | Agent identity ("name/version" or free-form). Optional — defaults to<br>the MCP caller's clientInfo when omitted. Providing this explicitly<br>lets an orchestrator claim on behalf of a sub-agent. |
| `id` | yes | string | Artifact ID to claim (e.g. `PRD-057`). Normalized to uppercase on disk. |
| `note` | no | string \| null | Optional free-form note surfaced by `forgeplan_claims --active`. |
| `ttl_minutes` | no | integer \| null | Time-to-live in minutes. Default 30, max 1440 (24h), min 1. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "agent": {
      "description": "Agent identity (\"name/version\" or free-form). Optional — defaults to\nthe MCP caller's clientInfo when omitted. Providing this explicitly\nlets an orchestrator claim on behalf of a sub-agent.",
      "type": [
        "string",
        "null"
      ]
    },
    "id": {
      "description": "Artifact ID to claim (e.g. `PRD-057`). Normalized to uppercase on disk.",
      "type": "string"
    },
    "note": {
      "description": "Optional free-form note surfaced by `forgeplan_claims --active`.",
      "type": [
        "string",
        "null"
      ]
    },
    "ttl_minutes": {
      "description": "Time-to-live in minutes. Default 30, max 1440 (24h), min 1.",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "workspace": {
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "required": [
    "id"
  ],
  "title": "ClaimParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
