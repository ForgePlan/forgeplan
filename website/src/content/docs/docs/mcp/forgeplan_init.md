---
title: forgeplan_init
description: "Initialize a new .forgeplan/ workspace. Creates LanceDB tables, config, and artifact subdirectories."
---

Bootstraps a fresh Forgeplan workspace in the current directory. The agent calls this as the first MCP invocation when it detects a project that doesn't yet have `.forgeplan/` — for example after `git clone` on a new machine or when scaffolding a brand-new project from scratch. MCP invocations are implicitly non-interactive (no prompts), equivalent to CLI `forgeplan init -y`.

**Category**: Workspace & Data

## When an agent calls this

- First MCP call on a fresh clone — no `.forgeplan/lance/` exists yet, agent needs a working index.
- Scaffolding a new project from zero — agent will chain `init` → `new prd` → `validate`.
- Recovery after a wiped workspace — user deleted `.forgeplan/` accidentally; agent re-creates structure and then calls `forgeplan_import` or `forgeplan_scan_import` to rebuild.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `force` | `bool` | no (default: `false`) | Force reinitialize even if workspace exists. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::InitParams`_

## Returns

A status JSON confirming the workspace was created. Reports whether a scan-import ran and how many artifacts were imported. If `force=false` and the directory already contains `.forgeplan/`, returns an error object the agent should surface to the user rather than silently overwrite.

Example response shape:

```json
{
  "ok": true,
  "workspace": "/abs/path/.forgeplan",
  "tables_created": ["artifacts", "links", "evidence"],
  "scan_imported": 0
}
```

## Example invocation

```json
{ "force": false, "scan": true }
```

With typical agent context:

> Agent just cloned a repo that tracks `.forgeplan/*/` markdown but not the derived LanceDB index. It runs init with `scan: true` to recreate the index from tracked files.

```json
{ "scan": true }
```

## Typical sequence

`forgeplan_init` is usually the very first MCP call in a fresh session, followed by `forgeplan_list` or `forgeplan_health` to verify the state. For destructive re-init, the agent should call `forgeplan_export` first, save the JSON, and only then pass `force: true`.

## CLI equivalent

- [`forgeplan init`](/docs/cli/init/) — same operation, interactive prompt skipped via `-y`

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_import`](/docs/mcp/forgeplan_import/) — restore from export JSON
- [`forgeplan_list`](/docs/mcp/forgeplan_list/) — verify workspace after init
