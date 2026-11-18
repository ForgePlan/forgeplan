---
title: forgeplan_list
description: "List artifact summaries with optional kind/status filters."
---

Returns a filtered inventory of artifacts in the workspace. This is the cheapest discovery call - agents use it to answer questions like "what PRDs are active?" or "how many stale ADRs do we have?" without pulling full bodies. For full-text discovery the agent should reach for `forgeplan_search` instead.

**Category**: Reading Artifacts

## When an agent calls this

- Session bootstrap: agent needs a quick view of what exists before deciding next action.
- Filtering by state: "show me all draft PRDs" before running a batch validation pass.
- Answering a user question like "how many RFCs have we shipped?" - no full bodies needed.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `kind` | `string` | no | Filter by kind (`prd`, `rfc`, `adr`, ...). |
| `status` | `string` | no | Filter by status (`draft`, `active`, `stale`, `superseded`, `deprecated`). |
| `workspace` | `string \| null` | no | Explicit absolute or tilde-expanded workspace path. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ListParams`_

## Returns

The JSON text payload is an object with `artifacts`, `total`, and
`_next_action`, not a bare array or a `count` field. Each summary has
`id`, `kind`, `status`, `title`, `id_canonical`, `id_display`, and optional
`slug`, `predicted_number`, `assigned_number`. It does not include `depth`,
`tags`, `updated_at`, or `body`; use `forgeplan_get` for full content.
There is no MCP `tag` filter. Nullable `kind`, `status`, and `workspace`
are optional inputs, not required keys whose value must be null.

## Example invocation

```json
{ "kind": "prd", "status": "active" }
```

With typical agent context:

> Agent starting a session; wants a quick snapshot of active work before reading any bodies.

```json
{ "status": "active" }
```

## Typical sequence

Usually the second or third call of a session - after `forgeplan_init`/`forgeplan_health` and before `forgeplan_get` on a specific ID. Agents often pipe the result into a user-facing summary without any further MCP calls.

## CLI equivalent

- [`forgeplan list`](/docs/cli/list/) - same filters, human-readable table output

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_get`](/docs/mcp/forgeplan_get/) - read full content of one artifact
- [`forgeplan_search`](/docs/mcp/forgeplan_search/) - full-text / semantic discovery
- [`forgeplan_health`](/docs/mcp/forgeplan_health/) - aggregated project state

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

List artifacts with optional kind/status filters. Returns ID, kind, status, and title for each artifact.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `kind` | no | string \| null | Filter by kind (optional) |
| `status` | no | string \| null | Filter by status (optional) |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "kind": {
      "default": null,
      "description": "Filter by kind (optional)",
      "type": [
        "string",
        "null"
      ]
    },
    "status": {
      "default": null,
      "description": "Filter by status (optional)",
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
  "title": "ListParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
