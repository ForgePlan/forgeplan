---
title: forgeplan_release
description: "Release an advisory artifact claim so another agent can claim it."
---

Removes the claim file at `.forgeplan/claims/<id>.yaml`. By default the call refuses
when a different agent holds the claim - pass `force: true` (the orchestrator's escape
hatch) to override after a sub-agent crash. Missing claim is a no-op (idempotent).
Holds the workspace lock for the duration of the write so concurrent claim/release
calls cannot interleave.

**Category**: Multi-agent

## When an agent calls it

- Worker finishes the artifact and frees the slot for the next dispatch round.
- Worker crashes / times out - orchestrator force-releases with `agent: null, force: true`.
- Mistaken claim: agent grabbed the wrong ID, releases immediately to retry.
- Cleanup at session end: walk active claims and release each one before exit.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID whose claim to release. |
| `agent` | `string \| null` | no | Explicit holder identity; otherwise use cached MCP handshake identity. Without either identity, non-forced release fails. The catalog's “required unless force” wording describes this handler condition, not a required JSON key. |
| `force` | `bool` | no (default `false`) | Force-release regardless of holder - orchestrator override for crashed sub-agents. |
| `workspace` | `string \| null` | no | Explicit absolute or tilde-expanded workspace path. |

_Schema source: `crates/forgeplan-mcp/src/types.rs::ReleaseParams`_

## Returns

The success example below is illustrative and omits additional workspace
trace fields. A claim is an advisory marker, not a lock on source-file edits.

```json
{
  "id": "PRD-057",
  "released": true,
  "force": false,
  "_next_action": "Released claim on `PRD-057`."
}
```

A holder mismatch without `force` returns an MCP tool error with text such
as `claim held by worker-2, not you` and a textual `_next_action` hint,
not an `{"ok":false,"error":...}` JSON object. Confirm that the holder
has actually stopped before choosing the force-release example.

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
2. [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) → worker records advisory intent for its next artifact.
3. Worker does the artifact / code work.
4. `forgeplan_release` → free the slot.
5. Orchestrator re-dispatches.

## CLI equivalent

[`forgeplan release <id>`](/docs/cli/) - same semantics.

## See also

- [`forgeplan_claim`](/docs/mcp/forgeplan_claim/) - acquire the claim
- [`forgeplan_claims`](/docs/mcp/forgeplan_claims/) - see who holds what
- [`forgeplan_dispatch`](/docs/mcp/forgeplan_dispatch/) - re-plan after release

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Release an active claim. Refuses if the claim is held by a different agent (use `force: true` to override — the orchestrator's escape hatch for a crashed sub-agent). Missing claim is a no-op (idempotent).

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `agent` | no | string \| null | Required unless `force=true`. |
| `force` | no | boolean | Force-release regardless of holder (orchestrator escape hatch). |
| `id` | yes | string |  |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "agent": {
      "description": "Required unless `force=true`.",
      "type": [
        "string",
        "null"
      ]
    },
    "force": {
      "default": false,
      "description": "Force-release regardless of holder (orchestrator escape hatch).",
      "type": "boolean"
    },
    "id": {
      "type": "string"
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
  "title": "ReleaseParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
