---
title: forgeplan_import
description: "Import artifacts and relations from a JSON export bundle. Set force=true to overwrite existing artifacts."
---

Replays artifact records and relations from an export JSON string into the selected workspace. It is useful for a fresh destination, CI seeding or recovery from a saved export. It does not restore every workspace file or guarantee that every supplied relation was written; inspect the returned counters and resulting graph.

**Category**: Workspace & Data

## When an agent calls this

- After `forgeplan init -y` on a fresh workspace - seed it from a backup or shared bundle.
- On a new machine with an empty workspace - initialize it and import the saved export. If a clone already tracks the artifact projections, rebuild the derived index with CLI `forgeplan reindex` instead of reinitializing.
- In CI - reproduce a deterministic artifact set before running integration tests against the MCP server.
- Disaster recovery - the workspace was wiped, but you have a recent `forgeplan_export` JSON on hand.

## Collision behavior and partial writes

- `force` defaults to `false` (including omitted/null). Existing artifact IDs are skipped, but **relations are still replayed**, including relations between existing artifacts.
- With `force: true`, an existing artifact is deleted through the projection helper and recreated. This can remove its old projection and relations. There is no interactive overwrite prompt; obtain authorization and export the destination first.
- The bundle is not a transaction. Earlier writes remain after a later artifact error; an overwrite can already have removed the previous artifact before recreation fails.
- Record IDs are preserved. The handler does not replay exported `created_at`, `updated_at` or `r_eff_score` fields, so this is not a byte-for-byte or timestamp-preserving restore.

## Safety notes

- Import changes the artifact index, projections and relations under the workspace write lock. Do not infer consent to `force: true` from an advisory hint.
- Local configuration, recovery receipts and phase/claim state are not in the export. Artifact bodies are included verbatim and can carry sensitive content.
- For a clone with tracked projections, use CLI `forgeplan reindex`; `scan-import` is not a post-import refresh. Do not delete or reinitialize a surviving workspace just to import a bundle.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `data` | `string` | yes | JSON export data as a string |
| `force` | `boolean \| null` | no | Overwrite existing artifacts (default: false) Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The payload contains `imported`, `skipped`, `relations_imported` and `_next_action`, without workspace trace fields. Overwrites count as `imported`; no `artifacts_inserted` / `artifacts_overwritten` breakdown is returned.

Pass the actual export JSON as the string `data`, not a filename. The parsed bundle must have an `artifacts` array. `version` is not validated by this handler. Missing/non-string artifact IDs are ignored without incrementing `skipped`; omitted fields fall back to values such as `kind: "note"`, `status: "draft"` and `depth: "standard"` before downstream creation checks.

Relations use `source`, `target`, `relation`; missing `relation` defaults to `informs`. Entries without usable endpoints are ignored. Exact duplicate or failed relation writes are not counted, and failures in the batch can be logged or reduced to `relations_imported: 0` rather than failing the MCP call. A success is therefore not proof that the entire graph was restored.

Invalid JSON or a missing `artifacts` array produces MCP error text. An empty array is accepted and imports no artifacts, but a supplied `relations` array can still change the graph.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "data": "{\"version\":1,\"artifacts\":[{\"id\":\"NOTE-001\",\"kind\":\"note\",\"status\":\"draft\",\"title\":\"Storage decision\",\"body\":\"## Decision\\nUse PostgreSQL for transactional storage.\"}],\"relations\":[]}"
}
```

## Typical sequence

```
forgeplan_init                    ← fresh workspace (creates tables + dirs)
forgeplan_import (data=bundle)    ← load the snapshot
forgeplan_list                    ← verify count
forgeplan_health                  ← confirm workspace shape
```

For an accidental deletion, try [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) or [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) while the receipt is available. A saved export is the longer-term fallback. Do not delete a surviving workspace merely to import a backup; first preserve its current state and choose whether existing IDs should be skipped or explicitly overwritten.

## CLI equivalent

- [`forgeplan import <file>`](/docs/cli/import/) - reads directly from a file path, more ergonomic for large bundles.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_export`](/docs/mcp/forgeplan_export/) - produce the bundle
- [`forgeplan_init`](/docs/mcp/forgeplan_init/) - create the workspace before importing
- [`forgeplan_list`](/docs/mcp/forgeplan_list/) - verify the import
