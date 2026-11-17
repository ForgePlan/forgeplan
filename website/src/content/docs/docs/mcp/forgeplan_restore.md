---
title: forgeplan_restore
description: "Restore a soft-deleted artifact from the most recent non-consumed receipt in `.forgeplan/trash/`. Works for delete (recreates row + moves projection back), supersede (resets status + drops link), and deprecate (resets status). Refuses if a different artifact with the same ID currently exists (manual resolution required). TTL default: 30 days from the destructive op."
---

Applies recovery for a specific artifact from its newest non-consumed receipt in `.forgeplan/trash/`. For a delete it recreates the row and attempts to return the projection; for supersede/deprecate it resets the stored status and title, and for supersede also attempts to remove the replacement edge. Captured relations are re-added where the other endpoint exists. Recovery requires an available receipt; restore starts a lazy 30-day purge, so do not rely on receipts surviving beyond that window.

**Category**: Lifecycle / Recovery

## When an agent calls it

- "Restore PRD-042" - the user selected a mistakenly deleted artifact.
- Undo a specific mistaken supersede/deprecate, after inspecting the receipt.
- Recover a known ID when [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) would select a different operation.

This tool is a mutation, **not a receipt audit, preview or dry-run**. Inspect the receipt on disk and the activity log before calling it; there is no preview parameter and no interactive confirmation.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to recover from the most recent non-consumed receipt. |
| `target_status` | `string \| null` | no | Issue #291: optional explicit override of the restored status. Default behaviour reads `prior_status` from the receipt snapshot (which captures the artifact's status at the moment of `_delete` / `_deprecate` / `_supersede`). Pass `target_status: "active"` to force a specific status - useful when the operator wants to restore to a different state than was captured.  Valid values: `"draft"`, `"active"`, `"deprecated"`. Unknown values are rejected with a structured error. Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The payload contains `restored`, `op_reversed` (`delete`, `supersede` or `deprecate`), `relations_restored`, `relations_skipped`, `projection_restored`, `restored_status`, `restored_status_source`, `warnings` and `_next_action`. This handler does not add workspace trace fields.

`relations_skipped` lists missing opposite endpoints; other relation-write problems appear in `warnings`. A success with warnings or `projection_restored: false` is possible. Missing/unreadable receipts, collisions and invalid overrides produce MCP error text, not an `ok: false` JSON object.

## Guards and side effects

- `id` selects the latest non-consumed receipt by display ID or recorded slug. It does not select a receipt ID or prove that the recorded destructive operation finished successfully.
- Omit `target_status` to recover the snapshot status. An override is trimmed/lowercased and must be `draft`, `active` or `deprecated`. If the row still exists, the override is checked against its current lifecycle state; even explicitly supplying the snapshot status can fail where omitting the override succeeds.
- A delete restore refuses any existing row with the same ID. For supersede/deprecate, an existing row with a different kind is refused; the snapshot title can overwrite a later title change.
- Recovery marks the receipt consumed. Relation or projection problems can be warnings rather than a rollback. If marking the receipt consumed fails, the tool returns an error **after the store was already restored**; inspect the state instead of blindly retrying or deleting a conflicting artifact.

## Limits in 0.38.0

- `restored_status_source` is intended to distinguish the snapshot from an override, but the handler compares the request with the already-applied result. It can report `receipt_snapshot` even when an override changed the captured status. Check `restored_status` and the original receipt rather than using that source field as proof.
- For supersede/deprecate, the remaining Markdown projection is not re-rendered, and a deprecation's body section is not removed. `projection_restored: true` on that path means the file was never moved, not that its status, links or body are current.
- A successful restore does not re-run activation validation, re-score dependents or reset advisory phase history. Inspect the stored record and on-disk projection before resuming work.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "PRD-001"
}
```

## Typical sequence

1. [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) - find when the destructive op happened.
2. `forgeplan_restore` - recover the specific artifact.
3. [`forgeplan_get`](/docs/mcp/forgeplan_get/) - verify body, status, relations.
4. Re-link any skipped relations manually if needed.

## CLI equivalent

[`forgeplan restore <id>`](/docs/cli/) - same recovery semantics.

## See also

- [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) - reverse the most recent destructive op (no ID needed)
- [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) - locate the receipt before restoring
- [`forgeplan_delete`](/docs/mcp/forgeplan_delete/) - the soft-delete this reverses
- [MCP overview](/docs/mcp/)
