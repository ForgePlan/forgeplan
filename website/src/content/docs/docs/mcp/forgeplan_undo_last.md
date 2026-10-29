---
title: forgeplan_undo_last
description: "Reverse the most recent destructive operation (delete / supersede / deprecate) - undo button for AI agents."
---

Walks the soft-delete trash newest-first, finds the most recent non-consumed receipt
within `within_hours`, and applies the same restore logic as
[`forgeplan_restore`](/docs/mcp/forgeplan_restore/). Use when the agent realizes
"the last thing I did was wrong" without needing to know the artifact ID.
Returns an error with guidance when no matching receipt exists - never guesses.

**Category**: Lifecycle / Recovery

## When an agent calls it

- Immediately after a misfired `forgeplan_delete` / `_supersede` / `_deprecate`.
- User says "undo that" without specifying which artifact.
- Recovering from an LLM hallucination that took a destructive action.
- Pair with [`forgeplan_activity_stats`](/docs/mcp/forgeplan_activity_stats/) - saw the
  unexpected destructive call, undo it.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `within_hours` | `integer` or `null` | no (default 24) | Omission/null uses 24 hours. The handler clamps provided integers to 1–720; the emitted schema exposes minimum 0 but not that clamp or upper bound. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::UndoLastParams`_

## Returns

Illustrative response excerpts, not a record of a recovery performed here. Success can also include workspace-trace and index-reconciliation fields.

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

The handler returns an error MCP `CallToolResult` containing text, not a JSON `ok: false` object. Example text prefix:

```text
No non-consumed destructive op in the last 24 hour(s).

_next_action: Expand the window: `forgeplan_undo_last within_hours=720`.
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
2. `forgeplan_undo_last` - reverse it.
3. Repeat the call to undo the previous op (each call consumes the newest non-consumed receipt).
4. Or switch to [`forgeplan_restore <id>`](/docs/mcp/forgeplan_restore/) once you know the specific ID.

## CLI equivalent

[`forgeplan undo-last`](/docs/cli/) - same receipt-selection operation. The selected CLI does not register `forgeplan undo`.

## See also

- [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) - restore a specific artifact by ID
- [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) - inspect the destructive-op timeline
- [`forgeplan_delete`](/docs/mcp/forgeplan_delete/) - the soft-delete this reverses
- [MCP overview](/docs/mcp/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Reverse the most recent destructive operation (delete, supersede, or deprecate) by reading the soft-delete trash and applying forgeplan_restore to the most recently written non-consumed receipt. Params: within_hours (default 24, max 720). If no matching receipt is found, returns an error with guidance; the tool never guesses.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `within_hours` | no | integer \| null | Time window (hours) to search for the last destructive op.<br>Default 24, max 720 (30 days). |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "within_hours": {
      "default": null,
      "description": "Time window (hours) to search for the last destructive op.\nDefault 24, max 720 (30 days).",
      "format": "uint32",
      "minimum": 0,
      "type": [
        "integer",
        "null"
      ]
    },
    "workspace": {
      "default": null,
      "description": "Optional explicit workspace directory for this call. When set, the MCP\nserver resolves the `.forgeplan/` projection from this path instead of\nthe server's startup CWD. Use this when the calling agent is running\nin a git worktree separate from the MCP server's launch directory.\nPRD-078 / ADR-015. Accepts an absolute or tilde-expanded path.",
      "type": [
        "string",
        "null"
      ]
    }
  },
  "title": "UndoLastParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
