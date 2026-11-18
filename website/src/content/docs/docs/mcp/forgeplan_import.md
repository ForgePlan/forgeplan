---
title: forgeplan_import
description: "Import artifact records and relations from a JSON export bundle. Use force=true to overwrite existing IDs."
---

Loads artifact records and relations from a `forgeplan_export` JSON bundle into the selected workspace using file-first projection helpers. Canonical Markdown is primary, not the LanceDB row. This is an artifact/relations transfer, not a full workspace backup: config, federation registry, runtime state, and all original metadata are not restored by this operation.

**Category**: Workspace & Data

## When an agent calls this

- After `forgeplan init -y` on a fresh workspace - seed it from a backup or shared bundle.
- On a new developer machine - clone the repo, run `forgeplan init`, then import the team's shared export.
- In CI - reproduce a deterministic artifact set before running integration tests against the MCP server.
- Disaster recovery - the workspace was wiped, but you have a recent `forgeplan_export` JSON on hand.

## Idempotency and collision behaviour

- **Default (`force=false`)**: existing artifacts with the same ID are **skipped**. The importer logs the count of skipped items so the agent can report what changed.
- **`force=true`**: attempts to replace existing IDs through delete/create helpers, then reapplies bundle relations. Inspect the result and canonical files; this is not an atomic whole-workspace restore.
- IDs are preserved, but the importer reconstructs a selected set of metadata fields; it does not preserve every exported field (for example, artifact tags are initialized empty).
- The operation is **not transactional across the whole bundle** in the classical sense - it writes artifacts one by one. If something fails midway, already-written artifacts remain. Always `forgeplan_export` a fresh backup before a `force=true` import.

## Safety notes

- `forgeplan_import` modifies the workspace. Always take a safety export first.
- The bundle excludes workspace configuration, but artifact text can itself contain secrets. Inspect and protect exported data; do not assume it is API-key-free.
- On a migrated workspace, canonical writes and index reconciliation are distinct. Inspect `index_pending`; use `forgeplan index` for Constellation maintenance when needed. `scan-import` discovers new foreign Markdown and is not an index-repair step.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `data` | `string` | yes | The export JSON as a string. Pass the full contents of a bundle produced by `forgeplan_export`. For large bundles, prefer the CLI which can read from a file path. |
| `force` | `bool` | no (default: `false`) | Overwrite artifacts that already exist by ID. Use with care. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ImportParams`_

## Returns

Illustrative response excerpts. The handler reports `imported`, `skipped`, and `relations_imported`, plus reconciliation and workspace-trace fields. It does not split inserts from overwrites.

```json
{
  "imported": 2,
  "skipped": 0,
  "relations_imported": 1
}
```

With `force=true`, overwritten artifacts count as `imported`; there is no separate overwrite count:

```json
{
  "imported": 2,
  "skipped": 0,
  "relations_imported": 1
}
```

## Example invocation

Fresh import into an empty workspace. The following is an empty valid bundle example; use the actual export bytes for a restore:

```json
{ "data": "{\"version\":1,\"artifacts\":[],\"relations\":[]}" }
```

Force-overwrite during a migration:

```json
{
  "data": "{\"version\":1,\"artifacts\":[],\"relations\":[]}",
  "force": true
}
```

## Typical sequence

```
forgeplan_init                    ← fresh workspace (creates tables + dirs)
forgeplan_import (data=bundle)    ← load the snapshot
forgeplan_list                    ← verify count
forgeplan_health                  ← confirm workspace shape
```

Recovery after an accident:

1. Keep an export outside `.forgeplan/` and preserve the damaged workspace before recovery.
2. Initialize a separate empty recovery directory rather than deleting the original.
3. Import the saved bundle into that explicit workspace, then inspect artifact bodies and relations. This procedure does not certify restoration of unexported state.

## CLI equivalent

- [`forgeplan import <file>`](/docs/cli/import/) - reads directly from a file path, more ergonomic for large bundles.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_export`](/docs/mcp/forgeplan_export/) - produce the bundle
- [`forgeplan_init`](/docs/mcp/forgeplan_init/) - create the workspace before importing
- [`forgeplan_list`](/docs/mcp/forgeplan_list/) - verify the import

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Import artifacts and relations from a JSON export bundle. Set force=true to overwrite existing artifacts.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `data` | yes | string | JSON export data as a string |
| `force` | no | boolean \| null | Overwrite existing artifacts (default: false) |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "data": {
      "description": "JSON export data as a string",
      "type": "string"
    },
    "force": {
      "default": null,
      "description": "Overwrite existing artifacts (default: false)",
      "type": [
        "boolean",
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
    "data"
  ],
  "title": "ImportParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
