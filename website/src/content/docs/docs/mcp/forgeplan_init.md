---
title: forgeplan_init
description: "Initialize a new .forgeplan/ workspace. Creates LanceDB tables, config, and artifact subdirectories."
---

Creates a fresh Forgeplan workspace under the MCP server's startup directory, without interactive prompts. It accepts only `force`; there is no MCP `scan` or `workspace` argument. A clone that already contains tracked `.forgeplan/` Markdown usually needs the CLI [`forgeplan reindex`](/docs/cli/reindex/), not a destructive reinitialization.

**Category**: Workspace & Data

## When an agent calls this

- First setup in a project that has no `.forgeplan/` directory.
- Scaffolding a new project from zero - agent will chain `init` → `new prd` → `validate`.
- Recovery into an empty project from a saved `forgeplan_export` bundle, followed by `forgeplan_import`.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `force` | `boolean \| null` | no | Force reinitialize even if workspace exists Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

`workspace` identifies the `.forgeplan/` directory and `message` describes the result; `_next_action` suggests the next call. If a workspace already exists and `force` is omitted or false, the server returns "Already initialized" without rebuilding its index.

`force: true` removes the existing `.forgeplan/` directory found by walking up from the server's startup directory before recreating a workspace at startup CWD. It may therefore remove an ancestor workspace, not just a directory in CWD. This destroys artifacts, the derived index and local workspace data. Save an export and local configuration separately before choosing it; it is not an index-refresh operation.

Initialization creates artifact directories with `.gitkeep`, `config.yaml`,
a template-only `secrets.env` and LanceDB tables. Forgeplan does not load
`secrets.env` automatically. This MCP handler does not run CLI onboarding,
scan/import existing documents, install marketplace skills/agents, configure a host,
download an embedding model or call an LLM. Distribution setup and model/provider
configuration remain separate prerequisites for features that need them.
Success is JSON in MCP text content (`workspace`, `message`, `_next_action`), without
`resolved_via` / `resolved_workspace`. Removal, filesystem initialization and
LanceDB failures surface as JSON-RPC errors; a failed reinitialization does not restore
the deleted directory.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "force": false
}
```

## Typical sequence

Use `forgeplan_init` for a fresh workspace, then `forgeplan_status` or `forgeplan_health` to inspect it. In an existing workspace, start with those read tools instead. Before a deliberately destructive re-init, obtain approval, save `forgeplan_export` JSON and preserve local files not covered by the export.

## CLI equivalent

- [`forgeplan init`](/docs/cli/init/) — fresh initialization, with `-y` / `--yes` for non-interactive defaults. **Force semantics differ:** CLI `--force` preserves existing artifact bodies and takes an automatic backup unless `--no-backup`; MCP `force: true` deletes the workspace. CLI `--scan` and `--with-model` have no counterparts in this MCP tool, and CLI `--yes` alone does not download a model.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_import`](/docs/mcp/forgeplan_import/) - restore from export JSON
- [`forgeplan_list`](/docs/mcp/forgeplan_list/) - verify workspace after init
