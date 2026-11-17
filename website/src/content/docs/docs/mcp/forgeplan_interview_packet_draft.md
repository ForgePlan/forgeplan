---
title: forgeplan_interview_packet_draft
description: "Experimental core 0.38 stub: returns not_implemented after workspace checks, regardless of marketplace-pack installation."
---

**Forgeplan 0.38.0 returns `not_implemented` after successful workspace resolution and locking.** Installing [`forgeplan-brownfield-pack`](/docs/marketplace/forgeplan-brownfield-pack/) does not replace or enable the core handler: it has no plugin-dispatch path. The install hint in its runtime description and response is not evidence that the MCP call can generate a packet.

Use the published `interview-packager` skill separately for interview preparation. Do not build an integration that expects this MCP tool to return questions.

**Category**: Other

## What it does today

Exactly this:

1. Resolves the workspace through the write path. Resolution failures can be JSON-RPC errors.
2. Acquires the workspace lock.
3. Returns the stub envelope, with the resolved workspace trace injected and a `_next_action` hint
   appended. This is a successful MCP text-content JSON result, not `isError: true`.
   The workspace path and resolution method below are illustrative:

```json
{
  "_next_action": "Fix: install plugin — see marketplace issue #79 (`forgeplan-brownfield-pack`).",
  "marketplace_issue": 79,
  "message": "Install forgeplan-brownfield-pack plugin to enable; or invoke via shell pipeline temporarily.",
  "resolved_via": "param",
  "resolved_workspace": "/work/project/.forgeplan",
  "status": "not_implemented"
}
```

It reads no artifacts for interview generation, writes no interview artifacts, and generates no questions. Workspace resolution may open the store; acquiring the lock can fail with a plain-text tool error.

## Separate marketplace workflow

The published `forgeplan-brownfield-pack` includes an `interview-packager` skill. It describes draft, prioritization, clustering and ingestion modes for Domain Owner interview packets. Those are agent-driven skill instructions, not an implementation injected into this MCP endpoint.

`domain` and `seed_ids` are accepted by the advertised input schema but are not read by the 0.38.0 core stub. Only workspace resolution, locking and the fixed response occur here.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `domain` | `string \| null` | no | Target domain or feature area the interview packet should cover. Used by the plugin to scope question generation. Schema default: `null`. |
| `seed_ids` | `array<string>` | no | Optional list of existing artifact ids (HYP / UC / INV) that seed the interview agenda. Empty means "discover from scratch". Schema default: `[]`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "domain": "billing",
  "seed_ids": [
    "HYP-001"
  ]
}
```

## Notes

- **Schema acceptance is not implementation.** The listed keys and optionality describe the current input contract, not a guarantee that a future implementation will preserve every behavior.
- **The stub takes the workspace lock** even though it creates no interview artifacts.
- **The stub echoes the resolved workspace** (`resolved_via`, `resolved_workspace`); do not infer that other tools emit the same trace.
- **The annotations mark it idempotent and not read-only.** That does not remove workspace-resolution or locking failures.
- **What to use instead, today:** the brownfield discovery protocol
  ([`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/)) is the live path for mapping an
  existing codebase, and the plugin's skills can be driven directly as the envelope's message
  suggests.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_interview_packet_ingest`](/docs/mcp/forgeplan_interview_packet_ingest/) - the ingest half of this workflow, also a stub
- [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) - the brownfield discovery protocol that works today
- [`forgeplan_hypothesis_status`](/docs/mcp/forgeplan_hypothesis_status/) - which parked hypotheses an interview should target
- [`forgeplan-brownfield-pack`](/docs/marketplace/forgeplan-brownfield-pack/) - separate interview workflow skills
