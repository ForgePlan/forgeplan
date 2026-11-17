---
title: forgeplan_undo_last
description: "Reverse the most recent destructive operation (delete, supersede, or deprecate) by reading the soft-delete trash and applying forgeplan_restore to the most recently written non-consumed receipt. Params: within_hours (default 24, max 720). If no matching receipt is found, returns an error with guidance; the tool never guesses."
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

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `within_hours` | `integer \| null` | no | Time window (hours) to search for the last destructive op. Default 24, max 720 (30 days). Schema default: `null`. `format`: `uint32`. `minimum`: `0`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The payload contains `restored`, `op_reversed`, `receipt_id`, `relations_restored`, `relations_skipped`, `projection_restored`, `warnings`, `resolved_workspace`, `resolved_via` and `_next_action`. Unlike `forgeplan_restore`, this handler does **not** return `restored_status` or `restored_status_source`.

No eligible receipt produces MCP error text such as `No non-consumed destructive op in the last 24 hour(s).`, not an `ok: false` JSON object. If applying the selected receipt fails, the tool reports that failure; it does not skip it and try an older operation.

## Selection and side effects

- `within_hours` defaults to 24 and is **clamped** to 1–720. A schema-valid 0 therefore searches one hour, not an empty window.
- Selection is workspace-wide, across agents: the newest parseable, non-consumed receipt in the window wins. It is not necessarily the caller's last action. Inspect activity/receipts first; use `forgeplan_restore` for a selected artifact ID.
- There is no preview or interactive confirmation. Each successful call consumes a receipt; a repeated call may undo a **different artifact**, so do not automatically loop or retry from `_next_action`.
- Recovery uses the same core implementation and [0.38.0 limitations as restore](/docs/mcp/forgeplan_restore/#limits-in-0380): relations can be skipped, projection problems can be warnings, and an error can follow an already-restored row if consuming the receipt fails.
- Undoing supersede/deprecate resets stored status/title but does not refresh the remaining Markdown projection or remove the deprecation body section. Inspect the row and file; `projection_restored: true` does not certify a current projection.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

1. Inspect the destructive-operation timeline and identify the intended workspace and receipt.
2. If the newest eligible receipt is the operation the user authorized reversing, call `forgeplan_undo_last`.
3. Read the restored artifact, projection and warnings.
4. To undo another operation, inspect and authorize it separately; for a known ID prefer [`forgeplan_restore`](/docs/mcp/forgeplan_restore/).

## CLI equivalent

[`forgeplan undo-last`](/docs/cli/undo-last/) - same trash-walking logic.

## See also

- [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) - restore a specific artifact by ID
- [`forgeplan_activity`](/docs/mcp/forgeplan_activity/) - inspect the destructive-op timeline
- [`forgeplan_delete`](/docs/mcp/forgeplan_delete/) - the soft-delete this reverses
- [MCP overview](/docs/mcp/)
