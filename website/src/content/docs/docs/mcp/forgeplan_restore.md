---
title: forgeplan_restore
description: "Restore a soft-deleted artifact from the most recent non-consumed receipt in the trash."
---

Reverses delete / supersede / deprecate using the latest eligible,
non-consumed receipt in `.forgeplan/trash/`. It restores canonical Markdown,
status and recoverable relations; legacy workspaces also restore their
projection row, while migrated workspaces report reconciliation separately.
Identity collisions can refuse recovery. Receipts expire after the default
30-day TTL; restoration is not a replacement for a separate backup.

**Category**: Lifecycle / Recovery

## When an agent calls it

- "Restore PRD-042" - user noticed the wrong artifact was deleted yesterday.
- After a `forgeplan_supersede` the agent realizes was wrong: restore the original.
- Surgical recovery when [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) would
  reverse the wrong operation - passing the ID is more precise than "the most recent".
- After inspecting a receipt separately: this tool performs restoration and may purge expired receipts; it has no read-only preview parameter.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to recover from the most recent non-consumed receipt. |
| `target_status` | `string \| null` | no | Override the receipt's prior status with `draft`, `active`, or `deprecated`; lifecycle guards still apply. |
| `workspace` | `string \| null` | no | Explicit absolute or tilde-expanded workspace path. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::RestoreParams`_

## Returns

The success examples below illustrate recovery fields, not recorded
measurements. Additional restored-status and index-publication fields may
be returned. Inspect `index_pending` on migrated workspaces.

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

No eligible receipt returns an MCP tool error with explanatory text and a
textual `_next_action` hint, not the old `ok`/`error` JSON envelope.
To inspect recent operations, call `forgeplan_activity` with
`{"tool":"forgeplan_delete,forgeplan_supersede,forgeplan_deprecate","since_hours":720}`.
That log query does not itself restore anything.

## Example invocation

```json
{ "id": "PRD-042" }
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

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Restore a soft-deleted artifact from the most recent non-consumed receipt in `.forgeplan/trash/`. Works for delete (recreates row + moves projection back), supersede (resets status + drops link), and deprecate (resets status). Refuses if a different artifact with the same ID currently exists (manual resolution required). TTL default: 30 days from the destructive op.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | yes | string | Artifact ID to recover from the most recent non-consumed receipt. |
| `target_status` | no | string \| null | Issue #291: optional explicit override of the restored status.<br>Default behaviour reads `prior_status` from the receipt snapshot<br>(which captures the artifact's status at the moment of `_delete`<br>/ `_deprecate` / `_supersede`). Pass `target_status: "active"` to<br>force a specific status — useful when the operator wants to<br>restore to a different state than was captured.<br><br>Valid values: `"draft"`, `"active"`, `"deprecated"`.<br>Unknown values are rejected with a structured error. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to recover from the most recent non-consumed receipt.",
      "type": "string"
    },
    "target_status": {
      "default": null,
      "description": "Issue #291: optional explicit override of the restored status.\nDefault behaviour reads `prior_status` from the receipt snapshot\n(which captures the artifact's status at the moment of `_delete`\n/ `_deprecate` / `_supersede`). Pass `target_status: \"active\"` to\nforce a specific status — useful when the operator wants to\nrestore to a different state than was captured.\n\nValid values: `\"draft\"`, `\"active\"`, `\"deprecated\"`.\nUnknown values are rejected with a structured error.",
      "type": [
        "string",
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
  "required": [
    "id"
  ],
  "title": "RestoreParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
