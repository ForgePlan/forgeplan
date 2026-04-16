---
title: forgeplan_supersede
description: "Supersede an artifact (active → superseded). Creates a supersedes link to the replacement and marks the original as terminal."
---

Marks an active artifact as superseded by a newer one. This is a **terminal** transition — once superseded, the artifact never goes back to active. The tool also creates a `supersedes` link automatically, so the decision history is preserved and traceable. Agents use this when a PRD/RFC/ADR is being replaced (not retired, not deleted) by a new, better version.

**Category**: Lifecycle

## When an agent calls this

- Redesign: new RFC-019 replaces RFC-018 — agent calls `supersede RFC-018 --by RFC-019`.
- Second iteration of a decision after new evidence contradicts the old one.
- Renaming/restructuring: create new artifact, then supersede the old so history stays linked.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to supersede. |
| `by` | `string` | yes | Replacement artifact ID. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::SupersedeParams`_

## Returns

The transition confirmation plus the auto-created link. Agents should read `from`/`to` to verify it actually moved, and check `link` to confirm the graph is consistent.

Example response shape:

```json
{
  "ok": true,
  "id": "RFC-018",
  "from": "active",
  "to": "superseded",
  "link": { "source": "RFC-019", "target": "RFC-018", "relation": "supersedes" }
}
```

## Example invocation

```json
{ "id": "RFC-018", "by": "RFC-019" }
```

With typical agent context:

> Team adopted a new retry strategy documented in RFC-019. Agent supersedes RFC-018.

```json
{ "id": "RFC-018", "by": "RFC-019" }
```

## Typical sequence

`forgeplan_new` (replacement) → `forgeplan_update` → `forgeplan_validate` PASS → `forgeplan_activate` the replacement → `forgeplan_supersede` the old one. Never supersede before the replacement is active — you'd leave an orphan.

## CLI equivalent

- [`forgeplan supersede`](/docs/cli/supersede/) — same operation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) — preceding step
- [`forgeplan_deprecate`](/docs/mcp/forgeplan_deprecate/) — retirement without replacement
- [`forgeplan_link`](/docs/mcp/forgeplan_link/) — manual link creation
