---
title: forgeplan_capture
description: "Capture a decision from conversation into a Note or ADR artifact. Auto-detects type: simple decisions become Notes, architectural decisions become ADRs."
---

Turns free-form conversation context into a persisted artifact — either a `Note` (tactical micro-decision) or an `ADR` (architectural decision record) based on auto-detection. The agent calls this when it realizes a real decision was just made in chat but nobody wrote it down yet. Capture is how Forgeplan recovers "tribal knowledge" that would otherwise be lost to scrollback.

**Category**: Creating Artifacts

## When an agent calls this

- User ends a discussion with "let's go with approach B then" — agent captures to preserve the rationale.
- End-of-session checkpoint: agent surveys the chat and captures decisions before context is lost.
- When `forgeplan_health` reports that recent commits lack linked decisions — retroactive capture.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `decision` | `string` | yes | The decision statement to capture. |
| `context` | `string` | no | Additional context. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::CaptureParams`_

## Returns

The created artifact ID and the kind the classifier chose. Agents should read back the `kind` because it reveals whether the decision was considered "heavy" (ADR) or "light" (Note) — useful for deciding whether to follow up with reasoning or just link it.

Example response shape:

```json
{
  "ok": true,
  "id": "ADR-012",
  "kind": "adr",
  "auto_detected": true,
  "title": "Use JWT over session cookies for mobile clients",
  "path": ".forgeplan/adrs/adr-012-use-jwt-for-mobile.md"
}
```

## Example invocation

```json
{ "context": "We decided to use JWT with 15m access tokens and 7d refresh tokens for mobile clients, because session cookies break on iOS PWA." }
```

With typical agent context:

> After a design discussion, agent captures the final decision so it survives future sessions.

```json
{ "to": "adr", "context": "Settled on pg_partman for time-partitioning the events table because it integrates with pg_cron cleanly." }
```

## Typical sequence

Conversation → `forgeplan_capture` → `forgeplan_validate` (for ADR) → `forgeplan_link` (to any related PRD/RFC) → `forgeplan_activate`. Capture is often paired with `forgeplan_link relation=informs` so the new record doesn't float as an orphan.

## CLI equivalent

- [`forgeplan capture`](/docs/cli/capture/) — same operation, piped text input

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_generate`](/docs/mcp/forgeplan_generate/) — richer from-scratch draft
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) — empty stub alternative
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) — attach to parent decisions
