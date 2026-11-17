---
title: forgeplan_list
description: "List artifacts with optional kind/status filters. Returns ID, kind, status, and title for each artifact."
---

Returns a filtered inventory of artifacts in the workspace. This is the cheapest discovery call - agents use it to answer questions like "what PRDs are active?" or "how many stale ADRs do we have?" without pulling full bodies. For full-text discovery the agent should reach for `forgeplan_search` instead.

**Category**: Reading Artifacts

## When an agent calls this

- Session bootstrap: agent needs a quick view of what exists before deciding next action.
- Filtering by state: "show me all draft PRDs" before running a batch validation pass.
- Answering a user question like "how many RFCs have we shipped?" - no full bodies needed.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `kind` | `string \| null` | no | Filter by kind (optional) Schema default: `null`. |
| `status` | `string \| null` | no | Filter by status (optional) Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is a JSON object encoded in MCP `content[0].text`: `artifacts`, `total`, and `_next_action`. Each summary contains `id`, `kind`, `status`, `title`, `id_canonical`, and `id_display`; `slug`, `predicted_number`, and `assigned_number` appear only when available. Summaries do not include `depth`, `tags`, timestamps, or body. Use `forgeplan_get` for those record details. `total` counts the filtered result, not the whole workspace. This call does not change artifact status.

Illustrative response (legacy artifacts without optional identity fields):

```json
{
  "total": 2,
  "artifacts": [
    { "id": "PRD-042", "kind": "prd", "status": "active", "title": "Auth system", "id_canonical": "prd-042", "id_display": "PRD-042" },
    { "id": "PRD-041", "kind": "prd", "status": "active", "title": "FPF rules", "id_canonical": "prd-041", "id_display": "PRD-041" }
  ],
  "_next_action": "2 active artifact(s). Check trust: `forgeplan_health`."
}
```

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "kind": "prd",
  "status": "active"
}
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
