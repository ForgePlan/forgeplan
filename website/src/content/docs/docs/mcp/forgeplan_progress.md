---
title: forgeplan_progress
description: "Show checkbox progress for artifacts. Parses markdown checkboxes (- [ ] / - [x]) and computes completion percentages."
---

Counts checked and unchecked Markdown task-list items for one or all artifacts. The MCP response reports per-artifact totals and rounded whole-number percentages plus aggregate checkbox counts; it does not return section groups. Checked boxes report authored state, not independently verified implementation.

**Category**: Quality

## When an agent calls it

- **Session resume** - inspect the PRD / RFC's recorded checkbox progress.
- **Sprint check-in** - inspect per-artifact counts; read the body to locate unfinished sections.
- **PR description generation** - quote the current completion % to communicate status.
- **After marking boxes** - confirm the parser saw your `- [x]` updates.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | no | Artifact ID. If omitted, shows progress for all artifacts that contain checkboxes. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ProgressParams`_

## Returns

Illustrative response excerpt, not an implementation-acceptance measurement.

```json
{
  "artifacts": [
    {
      "id": "RFC-006",
      "title": "Authentication design",
      "kind": "rfc",
      "total": 18,
      "completed": 12,
      "percent": 67.0
    }
  ],
  "total_checkboxes": 18,
  "total_completed": 12
}
```

## Example invocation

```json
{ "id": "RFC-006" }
```

Or for everything:

```json
{}
```

## Typical sequence

1. `forgeplan_progress` on a specific RFC - see where you left off.
2. Implement next unchecked item.
3. `forgeplan_update` to flip `- [ ]` → `- [x]`.
4. `forgeplan_progress` again - confirm the percent moved.

## CLI equivalent

```bash
forgeplan progress
forgeplan progress RFC-006
```

## See also

- [`forgeplan_estimate`](/docs/mcp/forgeplan_estimate/) - planned hours vs current progress.
- [`forgeplan_update`](/docs/mcp/forgeplan_update/) - mutate body to tick a box.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Show checkbox progress for artifacts. Parses markdown checkboxes (- [ ] / - [x]) and computes completion percentages.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | no | string \| null | Artifact ID (shows all artifacts with checkboxes if omitted) |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "default": null,
      "description": "Artifact ID (shows all artifacts with checkboxes if omitted)",
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
  "title": "ProgressParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
