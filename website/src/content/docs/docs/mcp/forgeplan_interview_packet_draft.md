---
title: forgeplan_interview_packet_draft
description: "experimental: draft an interview packet to validate brownfield hypotheses. STUB - the full implementation ships with the `forgeplan-brownfield-pack` plugin (marketplace issue #79). Without the plugin installed this tool echoes a `not_implemented` envelope so agents can detect the gap and surface the install hint."
---

**This tool is an experimental stub.** It is registered on the MCP surface, its parameter schema is
final, and it does nothing yet: the real implementation ships with the
[`forgeplan-brownfield-pack`](/docs/marketplace/forgeplan-brownfield-pack/) plugin (marketplace
issue #79). Calling it today returns a `not_implemented` envelope so an agent can detect the gap and
surface the install hint instead of silently getting an empty answer.

Do not build a workflow that depends on this returning a packet. Treat it as a capability probe and
a fixed contract for the eventual implementation.

**Category**: Other

## What it does today

Exactly this:

1. Resolves the workspace through the write path, which also validates that the workspace exists
   and produces the same "no workspace" error every other tool emits.
2. Acquires the workspace lock.
3. Returns the stub envelope, with the resolved workspace trace injected and a `_next_action` hint
   appended:

```json
{
  "_next_action": "Fix: install plugin — see marketplace issue #79 (`forgeplan-brownfield-pack`).",
  "marketplace_issue": 79,
  "message": "Install forgeplan-brownfield-pack plugin to enable; or invoke via shell pipeline temporarily.",
  "resolved_via": "param",
  "resolved_workspace": "/private/tmp/agent-enrichmcpf/.forgeplan",
  "status": "not_implemented"
}
```

It reads no artifacts, writes no artifacts, and generates no questions.

## What it will do

The plugin's `interview-packager` skill clusters parked hypotheses and open questions into
structured Domain Owner interview packets - the document a human answers so an agent can convert
inferred claims into verified or refuted ones. The parameters below exist to drive that packet:
`domain` scopes question generation, `seed_ids` seeds the agenda from artifacts the workspace
already has.

The schema is frozen now, ahead of the implementation, so that agents adopting the brownfield
workflow do not have to relearn the call when the plugin lands. That is also why the fields are
documented as accepted even though the stub never reads them.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `domain` | `string` | no | Target domain or feature area the packet should cover. Used by the plugin to scope question generation. Not read by the stub. |
| `seed_ids` | `string[]` | no | Existing artifact ids (HYP / UC / INV) that seed the interview agenda. Defaults to an empty list, which means "discover from scratch". Not read by the stub. |
| `workspace` | `string` | no | Explicit workspace directory for this call, so an agent in a git worktree can target its own `.forgeplan/` instead of the server's launch directory. Absolute or tilde-expanded path. Read by the stub. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::InterviewPacketDraftParams`_

## Example invocation

What a real call will look like:

```json
{
  "domain": "billing",
  "seed_ids": ["HYP-001", "HYP-004"]
}
```

Which today returns the `not_implemented` envelope above, with `"marketplace_issue": 79`.

## Notes

- **The schema is a contract, not a hint.** Field names and optionality are fixed by the frozen wire
  shape, so an integration written against this page keeps working once the stub is replaced by the
  real implementation.
- **The stub takes the workspace lock** even though it writes nothing. That is deliberate
  future-proofing so the plugin implementation inherits correct locking rather than adding it later.
- **The stub echoes the resolved workspace** (`resolved_via`, `resolved_workspace`). Few tools do
  this; here it is the cheapest way for an agent to prove which workspace the call was routed to
  before it commits to a brownfield session.
- **The MCP schema advertises it as idempotent and not read-only.** Calling it repeatedly is
  harmless, and it is not a read tool in the annotation sense.
- **What to use instead, today:** the brownfield discovery protocol
  ([`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/)) is the live path for mapping an
  existing codebase, and the plugin's skills can be driven directly as the envelope's message
  suggests.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_interview_packet_ingest`](/docs/mcp/forgeplan_interview_packet_ingest/) - the ingest half of this workflow, also a stub
- [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) - the brownfield discovery protocol that works today
- [`forgeplan_hypothesis_status`](/docs/mcp/forgeplan_hypothesis_status/) - which parked hypotheses an interview should target
- [`forgeplan-brownfield-pack`](/docs/marketplace/forgeplan-brownfield-pack/) - the plugin that will implement this tool
