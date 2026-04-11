---
title: forgeplan_import
description: "Import artifacts and relations from a JSON export bundle. Restores a full workspace snapshot produced by forgeplan_export. Set force=true to overwrite artifacts that already exist by ID."
---

Loads a JSON bundle produced by `forgeplan_export` into the current workspace. Every artifact in the bundle is inserted into LanceDB, its markdown body is re-projected to `.forgeplan/{kind}s/`, and every relation is re-established. The primary use cases are **restoring after a reinit**, **migrating between machines**, **seeding CI pipelines** with a reference workspace, and **recovering from accidental deletion** when a prior export exists.

**Category**: Workspace & Data

## When an agent calls this

- After `forgeplan init -y` on a fresh workspace — seed it from a backup or shared bundle.
- On a new developer machine — clone the repo, run `forgeplan init`, then import the team's shared export.
- In CI — reproduce a deterministic artifact set before running integration tests against the MCP server.
- Disaster recovery — the workspace was wiped, but you have a recent `forgeplan_export` JSON on hand.

## Idempotency and collision behaviour

- **Default (`force=false`)**: existing artifacts with the same ID are **skipped**. The importer logs the count of skipped items so the agent can report what changed.
- **`force=true`**: existing artifacts with the same ID are **overwritten** (metadata and body). Relations are re-applied; duplicates are de-duplicated by `(from, to, type)`.
- IDs in the bundle are preserved. If the workspace is empty, import is effectively a clean restore.
- The operation is **not transactional across the whole bundle** in the classical sense — it writes artifacts one by one. If something fails midway, already-written artifacts remain. Always `forgeplan_export` a fresh backup before a `force=true` import.

## Safety notes

- `forgeplan_import` modifies the workspace. Always take a safety export first.
- The bundle does **not** contain API keys — after import you still need to reconfigure `.forgeplan/config.yaml` (LLM provider, Hindsight, Orchestra).
- Derived indexes (`.forgeplan/lance/`) are rebuilt on-the-fly as artifacts are written; you don't need to run `forgeplan scan-import` afterwards.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `data` | `string` | yes | The export JSON as a string. Pass the full contents of a bundle produced by `forgeplan_export`. For large bundles, prefer the CLI which can read from a file path. |
| `force` | `bool` | no (default: `false`) | Overwrite artifacts that already exist by ID. Use with care. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ImportParams`_

## Returns

Counts plus any warnings the importer emitted:

```json
{
  "status": "ok",
  "artifacts_imported": 187,
  "artifacts_skipped": 0,
  "relations_imported": 312,
  "relations_skipped": 4,
  "force": false,
  "warnings": []
}
```

With `force=true`, the response distinguishes inserts from overwrites:

```json
{
  "status": "ok",
  "artifacts_inserted": 14,
  "artifacts_overwritten": 173,
  "relations_imported": 312,
  "force": true
}
```

## Example invocation

Fresh restore into an empty workspace:

```json
{ "data": "{\"version\":1,\"artifacts\":[…],\"relations\":[…]}" }
```

Force-overwrite during a migration:

```json
{
  "data": "{\"version\":1,\"artifacts\":[…],\"relations\":[…]}",
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

```
forgeplan_export → safety.json     (always keep one around)
…something goes wrong…
rm -rf .forgeplan && forgeplan init -y
forgeplan_import (data=safety.json)
```

## CLI equivalent

- [`forgeplan import <file>`](/docs/cli/import/) — reads directly from a file path, more ergonomic for large bundles.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_export`](/docs/mcp/forgeplan_export/) — produce the bundle
- [`forgeplan_init`](/docs/mcp/forgeplan_init/) — create the workspace before importing
- [`forgeplan_list`](/docs/mcp/forgeplan_list/) — verify the import
