---
title: forgeplan_list
description: "List artifacts with optional kind/status/tag filters. Returns ID, kind, status, and title for each artifact."
---

Returns a filtered inventory of artifacts in the workspace. This is the cheapest discovery call — agents use it to answer questions like "what PRDs are active?" or "how many stale ADRs do we have?" without pulling full bodies. For full-text discovery the agent should reach for `forgeplan_search` instead.

**Category**: Reading Artifacts

## When an agent calls this

- Session bootstrap: agent needs a quick view of what exists before deciding next action.
- Filtering by state: "show me all draft PRDs" before running a batch validation pass.
- Answering a user question like "how many RFCs have we shipped?" — no full bodies needed.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `kind` | `string` | no | Filter by kind (`prd`, `rfc`, `adr`, ...). |
| `status` | `string` | no | Filter by status (`draft`, `active`, `stale`, `superseded`, `deprecated`). |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ListParams`_

## Returns

A JSON array of artifact summaries — ID, kind, status, title, and a few frontmatter fields (depth, tags, updated_at). Bodies are intentionally omitted; use `forgeplan_get` when the full content is needed.

Example response shape:

```json
{
  "count": 3,
  "artifacts": [
    { "id": "PRD-042", "kind": "prd", "status": "active", "title": "Auth system", "depth": "standard" },
    { "id": "PRD-041", "kind": "prd", "status": "active", "title": "FPF rules", "depth": "deep" },
    { "id": "PRD-040", "kind": "prd", "status": "draft", "title": "Scoring intelligence", "depth": "standard" }
  ]
}
```

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

Usually the second or third call of a session — after `forgeplan_init`/`forgeplan_health` and before `forgeplan_get` on a specific ID. Agents often pipe the result into a user-facing summary without any further MCP calls.

## CLI equivalent

- [`forgeplan list`](/docs/cli/list/) — same filters, human-readable table output

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_get`](/docs/mcp/forgeplan_get/) — read full content of one artifact
- [`forgeplan_search`](/docs/mcp/forgeplan_search/) — full-text / semantic discovery
- [`forgeplan_health`](/docs/mcp/forgeplan_health/) — aggregated project state
