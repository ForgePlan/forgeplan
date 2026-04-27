---
title: forgeplan_restore
description: "Restore a soft-deleted artifact from the most recent non-consumed receipt in the trash."
---

Reverses a destructive operation (delete / supersede / deprecate) for a specific
artifact ID by reading `.forgeplan/trash/`. Recreates the LanceDB row, moves the
projection back into place, restores relations where the targets still exist, and
flips status back from `superseded` / `deprecated`. Refuses if a different artifact
with the same ID currently exists (manual resolution required). Receipts older than
the TTL (30 days by default, lazily purged) are unrecoverable.

**Category**: Lifecycle / Recovery

## When an agent calls it

- "Restore PRD-042" — user noticed the wrong artifact was deleted yesterday.
- After a `forgeplan_supersede` the agent realizes was wrong: restore the original.
- Surgical recovery when [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) would
  reverse the wrong operation — passing the ID is more precise than "the most recent".
- Auditing a trash receipt before committing to a real restore.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to recover from the most recent non-consumed receipt. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::RestoreParams`_

## Returns

```json
{
  "restored": "PRD-042",
  "op_reversed": "delete",
  "relations_restored": 3,
  "relations_skipped": [],
  "projection_restored": true,
  "warnings": [],
  "_next_action": "Restored `PRD-042` (reversed delete). 3 relation(s) restored. Verify with `forgeplan_get PRD-042`."
}
```

When some relation targets no longer exist:

```json
{
  "restored": "PRD-042",
  "op_reversed": "delete",
  "relations_restored": 2,
  "relations_skipped": ["EVID-099", "RFC-007"],
  "projection_restored": true,
  "warnings": [],
  "_next_action": "Restored `PRD-042` (reversed delete). 2 relation(s) restored, 2 skipped because targets no longer exist. Review with `forgeplan_get PRD-042` and re-link manually if needed."
}
```

When no receipt is found:

```json
{
  "ok": false,
  "error": "No non-consumed receipt found for `PRD-042`.",
  "_next_action": "Check `.forgeplan/trash/` contents or use `forgeplan_activity --tool forgeplan_delete,forgeplan_supersede,forgeplan_deprecate --since 720h` to see recent destructive ops. Receipts older than 30 days are purged."
}
```

## Example invocation

```json
{ "id": "PRD-042" }
```

## Typical sequence

1. [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) — find when the destructive op happened.
2. `forgeplan_restore` — recover the specific artifact.
3. [`forgeplan_get`](/docs/mcp/forgeplan_get/) — verify body, status, relations.
4. Re-link any skipped relations manually if needed.

## CLI equivalent

[`forgeplan restore <id>`](/docs/cli/) — same recovery semantics.

## See also

- [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) — reverse the most recent destructive op (no ID needed)
- [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) — locate the receipt before restoring
- [`forgeplan_delete`](/docs/mcp/forgeplan_delete/) — the soft-delete this reverses
- [MCP overview](/docs/mcp/)
