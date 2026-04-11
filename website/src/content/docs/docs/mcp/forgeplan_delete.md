---
title: forgeplan_delete
description: "Delete an artifact from LanceDB and remove its markdown projection file."
---

Permanently removes an artifact from the workspace — both the LanceDB row and the markdown file on disk. This is destructive and unrecoverable (without an export backup), so agents should strongly prefer lifecycle transitions (`supersede`, `deprecate`) over deletion. Only call `forgeplan_delete` when the user explicitly asks to remove a typo, a test artifact, or a duplicate.

**Category**: Editing Artifacts

## When an agent calls this

- User explicitly says "delete NOTE-099, I created it by mistake".
- Cleaning up a test artifact from `forge-smoke` runs.
- Removing an orphan created during an interrupted decompose flow.

Agents should NEVER call this to "retire" an active decision — use `forgeplan_supersede` or `forgeplan_deprecate` so history is preserved.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to delete. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::DeleteParams`_

## Returns

A confirmation object with the removed ID and the path of the markdown file that was unlinked. If the artifact had inbound links from other artifacts, the MCP server may refuse the delete and return an error listing the dependents so the agent can resolve them first.

Example response shape:

```json
{
  "ok": true,
  "deleted": "NOTE-099",
  "removed_path": ".forgeplan/notes/note-099-test.md"
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

- [`forgeplan delete`](/docs/cli/delete/) — same operation, prompts for confirmation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_deprecate`](/docs/mcp/forgeplan_deprecate/) — reversible retirement
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) — replacement with history
- [`forgeplan_export`](/docs/mcp/forgeplan_export/) — back up before destructive ops
