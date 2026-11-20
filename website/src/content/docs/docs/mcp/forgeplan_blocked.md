---
title: forgeplan_blocked
description: "Show unmet structural dependencies; active, deprecated, and superseded prerequisites are resolved."
---

Shows artifacts with unmet structural dependencies (`based_on`, `refines`, `supersedes`, `contradicts`). The handler treats existing `active`, `deprecated`, and `superseded` prerequisites as resolved. Its lookup is not a draft-only test: a `stale` prerequisite or missing target can also remain unresolved, despite the emitted catalog's “only draft” summary below.

**Category**: Dashboards & Graph

## When an agent calls it

- **Sprint planning** - determine which artifacts are ready to work on vs waiting on upstream decisions.
- **Before activation** - confirm an artifact has no unmet prerequisites before calling `forgeplan_activate`.
- **Bottleneck diagnosis** - identify the upstream draft that is blocking many downstream artifacts.
- **Session start** - quickly see "what's ready to pick up" after restoring context.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | no | Artifact ID to check. If omitted, lists all blocked artifacts. |

The complete registered input contract, including optional nullable `workspace`, is below. Omitting `id` checks the whole workspace.

## Returns

Illustrative response excerpt for one blocked artifact. `blocked_by` is an array of IDs, not dependency objects.

```json
{
  "blocked": [
    {
      "id": "RFC-006",
      "blocked_by": ["PRD-039"]
    }
  ],
  "ready_count": 0,
  "blocked_count": 1,
  "cycles": []
}
```

## Example invocation

```json
{}
```

Or for a single artifact:

```json
{ "id": "RFC-006" }
```

## Typical sequence

1. `forgeplan_blocked` - list blocked artifacts.
2. `forgeplan_get` on the upstream `blocked_by` - check what it needs.
3. Review the upstream artifact and satisfy its activation gates before `forgeplan_activate`; validation alone is not the whole activation contract.
4. `forgeplan_blocked` again - confirm downstream is now unblocked.

## CLI equivalent

```bash
forgeplan blocked
forgeplan blocked RFC-006
```

## See also

- [`forgeplan_order`](/docs/mcp/forgeplan_order/) - full topological sort of the dependency graph.
- [`forgeplan_graph`](/docs/mcp/forgeplan_graph/) - visualize the graph.
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - unblock downstream by activating a draft.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Show blocked artifacts and their unmet dependencies. Only draft artifacts block — deprecated and superseded are considered resolved. Uses structural relations only (based_on, refines, supersedes, contradicts).

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | no | string \| null | Optional artifact ID to check. If omitted, shows all blocked artifacts. |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Optional artifact ID to check. If omitted, shows all blocked artifacts.",
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
  "title": "BlockedParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
