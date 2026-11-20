---
title: forgeplan_delete
description: "Soft-delete an artifact with a recovery receipt and move its Markdown projection to trash."
---

Soft-deletes an artifact from the workspace. The server records a recovery receipt and moves the canonical Markdown projection into `.forgeplan/trash/`; it removes the derived row only on a legacy-owned index. Recovery uses `forgeplan_undo_last` or `forgeplan_restore`, subject to receipt availability, expiry, and collision checks. Prefer lifecycle transitions (`supersede`, `deprecate`) for retiring decisions. Only call `forgeplan_delete` when the user explicitly asks to remove a typo, a test artifact, or a duplicate.

**Category**: Editing Artifacts

## When an agent calls this

- User explicitly says "delete NOTE-099, I created it by mistake".
- Cleaning up a test artifact from `forge-smoke` runs.
- Removing an orphan created during an interrupted decompose flow.

Agents should NEVER call this to "retire" an active decision - use `forgeplan_supersede` or `forgeplan_deprecate` so history is preserved.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to delete. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::DeleteParams`_

## Returns

The response identifies the artifact and recovery receipt. The handler captures its relations for recovery; it does not implement the inbound-dependency refusal previously described here. On a migrated workspace, inspect the reconciliation fields such as `index_pending` separately from the canonical deletion.

Illustrative response excerpt:

```json
{
  "id": "NOTE-099",
  "title": "Test note",
  "message": "Soft-deleted — recoverable via forgeplan_undo_last",
  "receipt_id": "illustrative-receipt-id"
}
```

## Example invocation

```json
{ "id": "NOTE-099" }
```

With typical agent context:

> User says "that PROB-099 was a mistake, nuke it". Agent confirms with the user, then deletes.

```json
{ "id": "PROB-099" }
```

## Typical sequence

`forgeplan_list` or `forgeplan_get` to confirm the target → explicit user confirmation → `forgeplan_delete` → `forgeplan_health` to verify nothing broke. For reversible retirement the flow is `forgeplan_deprecate` (active → deprecated) instead.

## CLI equivalent

- [`forgeplan delete`](/docs/cli/delete/) - same operation, prompts for confirmation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_deprecate`](/docs/mcp/forgeplan_deprecate/) - reversible retirement
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) - replacement with history
- [`forgeplan_export`](/docs/mcp/forgeplan_export/) - back up before destructive ops

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Delete an artifact: writes a soft-delete receipt, moves its markdown projection into `.forgeplan/trash/`, and drops the legacy projection row on workspaces whose index the legacy store still owns.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | yes | string | Artifact ID to delete |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to delete",
      "type": "string"
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
  "required": [
    "id"
  ],
  "title": "DeleteParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
