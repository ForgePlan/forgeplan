---
title: forgeplan_progress
description: "Show checkbox progress for artifacts. Parses markdown checkboxes (- [ ] / - [x]) and computes completion percentages."
---

Show checkbox completion for one or all artifacts from each stored body's own task-list lines. There are no section groups or child-artifact rollups. The parser recognises indented `-`/`*` list markers followed by `[ ]`, `[x]`, or `[X]` and a trailing space; it does not exclude fenced code examples.

**Category**: Quality

## When an agent calls it

- **Session resume** - see how much of the PRD / RFC you've actually implemented.
- **Sprint check-in** — compare checkbox totals by artifact, not by section or implementation phase.
- **PR description generation** - quote the current completion % to communicate status.
- **After marking boxes** - confirm the parser saw your `- [x]` updates.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string \| null` | no | Artifact ID (shows all artifacts with checkboxes if omitted) Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP `content[0].text` with `artifacts`, `total_checkboxes`, `total_completed`, and `_next_action`. Each item has `id`, `title`, `kind`, `total`, `completed`, and `percent` rounded to the nearest whole percentage. There is no `done`, `groups`, or top-level percent.

Omitting `id` excludes records without checkboxes. An explicit display ID is case-insensitive and includes a zero-checkbox record with `percent: 0.0`; a missing ID is a plain-text tool error (`isError: true`). The tool never ticks boxes, activates artifacts, or verifies implementation. Its activation hint does not bypass lifecycle checks.

Illustrative partial payload (the dynamic `_next_action` is omitted):

```json
{
  "artifacts": [
    {"id": "RFC-006", "title": "Auth implementation", "kind": "rfc", "total": 18, "completed": 12, "percent": 67.0}
  ],
  "total_checkboxes": 18,
  "total_completed": 12
}
```

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

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
