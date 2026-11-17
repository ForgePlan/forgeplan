---
title: forgeplan_delete
description: "Delete an artifact from LanceDB and remove its markdown projection file."
---

Soft-deletes an artifact: captures a receipt in `.forgeplan/trash/`, attempts to move its Markdown projection there, then removes the store record. The receipt supports recovery through [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) or [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/), while it remains available (normally within 30 days) and collision checks permit recovery. This is still destructive. Prefer lifecycle transitions for retirement, and delete only an explicitly selected unwanted artifact.

**Category**: Editing Artifacts

## When an agent calls this

- User explicitly says "delete NOTE-099, I created it by mistake".
- Cleaning up a test artifact from `forge-smoke` runs.
- Removing an orphan created during an interrupted decompose flow.

Agents should NEVER call this to "retire" an active decision - use `forgeplan_supersede` or `forgeplan_deprecate` so history is preserved.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to delete |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The confirmation contains `id`, `title`, `message`, `receipt_id`, the workspace trace (`resolved_workspace`, `resolved_via`) and `_next_action`. It does not return `deleted` or `removed_path`.

## Guards and side effects

- The target must exist, but no lifecycle-status gate prevents deletion of an active decision. The MCP call has no `yes` flag or interactive confirmation: obtain explicit consent for the selected ID before calling.
- The handler holds the workspace write lock. Receipt capture happens before store mutation; if writing the receipt fails, the deletion aborts.
- **0.38.0 limitation:** moving a missing/unmovable projection is best-effort inside receipt capture. The row can be deleted even if the original Markdown file remains. Success is not proof that the projection disappeared.
- The post-capture helper deletes the artifact row, not all graph relations or dependent projections. Do not assume incoming references or dependent scores were cleaned up. Inspect the graph and retain an export for longer-term backup.
- An error after receipt creation can follow partial side effects. Inspect the current row, projection and receipt before retrying or applying recovery.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "NOTE-001"
}
```

## Typical sequence

`forgeplan_list` or `forgeplan_get` to confirm the target → explicit user confirmation → `forgeplan_delete` → `forgeplan_health` to verify nothing broke. For reversible retirement the flow is `forgeplan_deprecate` (active → deprecated) instead.

## CLI equivalent

- [`forgeplan delete`](/docs/cli/delete/) - same operation, prompts for confirmation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) - recover a specific artifact from its receipt
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) - replacement with history
- [`forgeplan_export`](/docs/mcp/forgeplan_export/) - back up before destructive ops
