---
title: forgeplan_update
description: "Update artifact metadata (status, title) and/or body. Re-renders markdown projection after update."
---

Patches an existing artifact's body and/or frontmatter metadata in-place. This is the direct-edit path — the agent uses it to fill a freshly created stub, rewrite a section after user feedback, or rename an artifact. For lifecycle transitions (draft → active → superseded) the agent must use the dedicated `forgeplan_activate` / `forgeplan_supersede` / `forgeplan_deprecate` tools instead, which enforce validation gates.

**Category**: Editing Artifacts

## When an agent calls this

- Immediately after `forgeplan_new` to populate MUST sections (Problem, Goals, Non-Goals, FR).
- Applying a user-requested edit: "add a section on observability to RFC-007".
- Fixing a validator finding: `forgeplan_validate` reports a missing section, agent writes it and re-validates.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to update. |
| `status` | `string` | no | New status: `draft`, `active`, `superseded`, `deprecated`. |
| `title` | `string` | no | New title. |
| `body` | `string` | no | New body content (full markdown replacement). |

_Schema source: `crates/forgeplan-mcp/src/server.rs::UpdateParams`_

## Returns

The updated artifact record with new `updated_at`. The markdown projection on disk is re-rendered automatically. If `status` is set to a value that requires a lifecycle gate (e.g. `active`), the call is rejected — the agent must use `forgeplan_activate` instead.

Example response shape:

```json
{
  "ok": true,
  "id": "PRD-042",
  "updated_at": "2026-04-11T10:02:14Z",
  "changed": ["title", "body"]
}
```

## Example invocation

```json
{ "id": "PRD-042", "title": "Authentication and session system" }
```

With typical agent context:

> Agent just ran `forgeplan_new`, got `PRD-042`, and now fills the MUST sections in a single body replacement.

```json
{ "id": "PRD-042", "body": "# PRD-042: Authentication system\n\n## Problem\n..." }
```

## Typical sequence

`forgeplan_new` → `forgeplan_update` (fill stub) → `forgeplan_validate` → loop on any findings → `forgeplan_activate`. For late edits to an active artifact, the pattern is `forgeplan_get` → compute patch → `forgeplan_update`.

## CLI equivalent

- Direct markdown edits (no single-command equivalent — CLI users edit `.forgeplan/*/*.md` and rely on auto-projection).

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) — the correct path for `draft → active`
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) — verify after the edit
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) — replace instead of edit
