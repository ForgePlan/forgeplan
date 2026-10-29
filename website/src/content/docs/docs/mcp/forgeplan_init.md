---
title: forgeplan_init
description: "Initialize a new .forgeplan/ workspace. Creates LanceDB tables, config, and artifact subdirectories."
---

Bootstraps a fresh Forgeplan workspace under the server's workspace root, without prompts. This tool registers only optional nullable `force`; the handler does not use `scan` or a per-call `workspace` selector. Do not confuse it with CLI initialization or use it merely because a derived index is missing.

**Category**: Workspace & Data

## When an agent calls this

- Initialize a genuinely new project with no canonical `.forgeplan/` workspace.
- For a clone with tracked artifact Markdown, preserve those files and use the appropriate index-maintenance path: `forgeplan index` for Constellation, `forgeplan reindex` for a legacy layout.
- For recovery, preserve original files and restore from a backup into a separate workspace. There is no registered `forgeplan_scan_import` MCP tool.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `force` | `bool` or `null` | no (default: `false`) | **Destructive in MCP**: remove the existing workspace directory and recreate it. Export and preserve a backup first. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::InitParams`_

## Returns

Returns `workspace` and `message`, with a next-action hint. If a workspace already exists and `force` is false or omitted, the handler returns an “Already initialized” message rather than the error previously described here. It does not scan/import tracked Markdown.

Illustrative response excerpt:

```json
{
  "workspace": "/abs/path/.forgeplan",
  "message": "Initialized .forgeplan/ for project 'example'"
}
```

## Example invocation

```json
{ "force": false }
```

With typical agent context:

> For a genuinely new project, initialize without force. For an existing clone, use index maintenance rather than adding an unsupported `scan` input.

```json
{}
```

## Typical sequence

On a new workspace: `forgeplan_init` → `forgeplan_list` or `forgeplan_health` to inspect state. Do not use `force: true` as routine repair: this MCP handler removes the existing `.forgeplan/` directory before recreating it.

## CLI equivalent

- [`forgeplan init`](/docs/cli/init/) is related but **not equivalent**: the selected CLI documents additive `--force` with an automatic backup by default, while this MCP handler deletes the existing workspace. CLI `--scan` is not an MCP input.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_import`](/docs/mcp/forgeplan_import/) - restore from export JSON
- [`forgeplan_list`](/docs/mcp/forgeplan_list/) - verify workspace after init

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Initialize a new .forgeplan/ workspace. Creates LanceDB tables, config, and artifact subdirectories.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `force` | no | boolean \| null | Force reinitialize even if workspace exists |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "force": {
      "default": null,
      "description": "Force reinitialize even if workspace exists",
      "type": [
        "boolean",
        "null"
      ]
    }
  },
  "title": "InitParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
