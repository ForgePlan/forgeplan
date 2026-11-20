---
title: forgeplan_export
description: "Export artifact records and relations to JSON inline or to a destination file."
---

Exports artifact record fields and relations from the selected workspace as JSON. This is not a byte-for-byte backup of the workspace directory: auxiliary state, logs, claims, config and caches are outside the bundle. A migrated workspace's records come from a captured canonical Markdown view; a legacy workspace uses its projection.

**Category**: Workspace & Data

## When an agent calls this

- **Backup** before a risky operation such as `forgeplan init` re-initialisation, a schema upgrade, or bulk deletes - the #1 guardrail against losing artifacts.
- **Cross-machine transfer** - hand an encrypted JSON blob to another dev's machine and restore via `forgeplan_import`.
- **CI seeding** - snapshot a reference workspace once and load it into each pipeline run deterministically.
- **Audit / compliance** - freeze the state of decisions at a release boundary for traceability.
- **Debugging ForgePlan itself** - attach the export to a bug report so maintainers can reproduce the exact workspace state.

## Safety notes

- Export does not modify artifact records. Supplying `output` **writes or overwrites a file**: an absolute destination is used as-is; a relative one is resolved under the project root. Choose a backup destination, not a canonical source path.
- Config and provider keys stored there are not included as separate files. Artifact bodies are exported verbatim and can themselves contain sensitive information; review the bundle before sharing.
- Derived indexes and model caches are not included. `forgeplan index` publishes Constellation; legacy `forgeplan reindex` rebuilds the legacy projection. `scan-import` imports foreign Markdown and is not an index-recovery command.
- The export is the canonical backup format. Never `rm -rf .forgeplan` without calling `forgeplan_export` first.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `output` | `string` | no | Destination file path. If omitted, the tool returns the JSON bundle inline in the response. If provided, the tool writes to disk and returns a small success summary. |
| `workspace` | `string \| null` | no | Explicit absolute or tilde-expanded workspace path. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ExportParams`_

## Returns

With `output`, the success payload has `written_to`, numeric `artifacts`
and `relations` counts, and `_next_action`. It has no `bytes` field.

Without `output`, it has `version: 1`, `artifacts`, `relations`, and
`_next_action`; there is no `generated_at` field. Artifact entries contain
`id`, `kind`, `status`, `title`, `body`, `depth`, `author`, `parent_epic`,
`r_eff_score`, `valid_until`, `created_at`, `updated_at` (not `tags`).
Relation entries use `source`, `target`, `relation`, not `from`/`to`/`type`.
The export does not encrypt its output.

Inline mode is convenient for small workspaces; for large ones prefer writing to a file to avoid bloating MCP response payloads.

## Example invocation

Write to file (recommended for backups):

```json
{ "output": "/tmp/forgeplan-backup-2026-04-11.json" }
```

Return inline (for programmatic consumption):

```json
{}
```

## Typical sequence

```
forgeplan_export (output=backup.json)       ← safety snapshot
…perform risky operation…
forgeplan_import (data=<contents of backup.json>)   ← restore if things go wrong
```

For migration:

```
[machine A] forgeplan_export → bundle.json
[transfer bundle.json]
[machine B] forgeplan init → forgeplan_import (data=<JSON contents of bundle.json, not its filename>)
```

## CLI equivalent

- [`forgeplan export`](/docs/cli/export/) - terminal export; do not assume its wrapper fields match the MCP payload.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_import`](/docs/mcp/forgeplan_import/) - restore from the bundle
- [`forgeplan_init`](/docs/mcp/forgeplan_init/) - initialise workspace before import

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Export all artifacts and relations to a JSON bundle. Returns the exported data directly for programmatic use, or writes to a file path.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `output` | no | string \| null | Optional output file path. If omitted, returns JSON directly. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "output": {
      "default": null,
      "description": "Optional output file path. If omitted, returns JSON directly.",
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
  "title": "ExportParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
