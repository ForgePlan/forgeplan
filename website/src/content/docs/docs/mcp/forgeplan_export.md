---
title: forgeplan_export
description: "Export all artifacts and relations to a JSON bundle. Returns the exported data directly for programmatic use, or writes to a file path."
---

Produces a JSON bundle of stored artifact records, their Markdown bodies and the relation graph. It is an artifact-data export, not a complete backup of every workspace file. Return it inline for programmatic use or write it to an explicitly selected path; review the content before sharing it.

**Category**: Workspace & Data

## When an agent calls this

- **Backup** before a risky operation such as `forgeplan init` re-initialisation, a schema upgrade, or bulk deletes - the #1 guardrail against losing artifacts.
- **Cross-machine transfer** - hand an encrypted JSON blob to another dev's machine and restore via `forgeplan_import`.
- **CI seeding** - snapshot a reference workspace once and load it into each pipeline run deterministically.
- **Audit / compliance** - freeze the state of decisions at a release boundary for traceability.
- **Debugging ForgePlan itself** - attach the export to a bug report so maintainers can reproduce the exact workspace state.

## Safety notes

- Reading artifacts does not change them, but `output` writes a file and overwrites an existing destination without prompting. Relative paths resolve from the project directory containing the selected `.forgeplan/`; absolute paths are accepted. Parent directories must already exist.
- Local config/provider credentials, advisory phases, claims, discovery sessions, recovery receipts and derived indexes are not included. Artifact bodies are exported verbatim and may themselves contain sensitive information.
- The handler reads records and relations without a mutation lock; do not assume an atomic snapshot while other writers are active.
- Use CLI `forgeplan reindex` to rebuild a derived artifact index from tracked projections. `scan-import` imports arbitrary Markdown, not this export bundle.
- Preserve the export and separately required local files before a destructive operation. A returned hint is not permission to commit a sensitive bundle, reinitialize a surviving workspace or run an import automatically.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `output` | `string \| null` | no | Optional output file path. If omitted, returns JSON directly. Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

With `output`, the payload contains `written_to` (resolved destination), `artifacts` and `relations` (counts), plus `_next_action`.

Without `output`, it contains `version: 1`, `artifacts` and `relations` arrays, plus `_next_action`. This version is the export format, not the Forgeplan release. Each artifact object has `id`, `kind`, `status`, `title`, `body`, `depth`, `author`, `parent_epic`, `r_eff_score`, `valid_until`, `created_at` and `updated_at`. Relation objects have `source`, `target` and `relation`.

Neither path adds workspace trace fields. Store/read/write failures are errors, not an empty successful bundle. For large workspaces prefer a file to avoid an oversized MCP text response.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

Backup: call `forgeplan_export` with a chosen `output` → preserve the bundle and separately required workspace files → perform only the authorized operation.

For migration, initialize only a genuinely fresh destination, read the saved bundle and pass its **JSON contents as the `data` string** to `forgeplan_import`. If a clone already contains tracked artifact projections, use CLI `forgeplan reindex` instead of reinitializing it.

## CLI equivalent

- [`forgeplan export`](/docs/cli/export/) - identical behaviour from the terminal.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_import`](/docs/mcp/forgeplan_import/) - restore from the bundle
- [`forgeplan_init`](/docs/mcp/forgeplan_init/) - initialise workspace before import
