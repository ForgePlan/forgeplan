---
title: forgeplan_undo_last
description: "Reverse the most recent destructive operation (delete / supersede / deprecate) — undo button for AI agents."
---

Walks the soft-delete trash newest-first, finds the most recent non-consumed receipt
within `within_hours`, and applies the same restore logic as
[`forgeplan_restore`](/docs/mcp/forgeplan_restore/). Use when the agent realizes
"the last thing I did was wrong" without needing to know the artifact ID.
Returns an error with guidance when no matching receipt exists — never guesses.

**Category**: Lifecycle / Recovery

## When an agent calls it

- Immediately after a misfired `forgeplan_delete` / `_supersede` / `_deprecate`.
- User says "undo that" without specifying which artifact.
- Recovering from an LLM hallucination that took a destructive action.
- Pair with [`forgeplan_activity_stats`](/docs/mcp/forgeplan_activity_stats/) — saw the
  unexpected destructive call, undo it.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `within_hours` | `number` | no (default 24, max 720) | Time window to search for the last destructive op. Expand to 720 (30 days) when in doubt. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::UndoLastParams`_

## Returns

```json
{
  "restored": "PRD-042",
  "op_reversed": "delete",
  "receipt_id": "trash-2026-04-26T10-14-22-001",
  "relations_restored": 3,
  "relations_skipped": [],
  "projection_restored": true,
  "warnings": [],
  "_next_action": "Reversed most recent delete of `PRD-042`. To undo another, call `forgeplan_undo_last` again (finds the next newest non-consumed receipt). Or restore a specific ID: `forgeplan_restore <id>`."
}
```

When nothing to undo in the window:

```json
{
  "ok": false,
  "error": "No non-consumed destructive op in the last 24 hour(s).",
  "_next_action": "Expand the window: `forgeplan_undo_last within_hours=720`. Or inspect the log: `forgeplan_activity --tool forgeplan_delete,forgeplan_supersede,forgeplan_deprecate --since 720h`."
}
```

## Example invocation

Default 24 h window:

```json
{}
```

Wider search after an idle period:

```json
{ "within_hours": 720 }
```

## Typical sequence

1. Misfire happens (`forgeplan_delete`, `_supersede`, or `_deprecate`).
2. `forgeplan_undo_last` — reverse it.
3. Repeat the call to undo the previous op (each call consumes the newest non-consumed receipt).
4. Or switch to [`forgeplan_restore <id>`](/docs/mcp/forgeplan_restore/) once you know the specific ID.

## CLI equivalent

[`forgeplan undo`](/docs/cli/) — same trash-walking logic.

## See also

- [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) — restore a specific artifact by ID
- [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) — inspect the destructive-op timeline
- [`forgeplan_delete`](/docs/mcp/forgeplan_delete/) — the soft-delete this reverses
- [MCP overview](/docs/mcp/)
