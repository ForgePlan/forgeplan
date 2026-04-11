---
title: forgeplan_get
description: "Read a full artifact by ID. Returns all metadata and body content."
---

Returns a single artifact's full markdown body plus frontmatter metadata. This is the canonical "read" operation — the agent calls it when it needs the actual contents of a PRD/RFC/ADR to reason about, quote, or update. Unlike `forgeplan_list`, the response contains everything the agent needs to understand the artifact.

**Category**: Reading Artifacts

## When an agent calls this

- User asks "what does PRD-042 say about rate limits?" — agent fetches then quotes the relevant section.
- Before calling `forgeplan_update` — the agent needs the current body to produce a diff-aware patch.
- Before `forgeplan_reason` — agent pre-reads so it can describe the context to the user if the LLM call is slow.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to read (e.g. `PRD-042`, case-insensitive). |

_Schema source: `crates/forgeplan-mcp/src/server.rs::GetParams`_

## Returns

The artifact as a JSON object: all frontmatter fields (kind, status, depth, tags, dates, valid_until) plus the markdown body as a string. If the ID doesn't exist, returns an error so the agent can recover (often by calling `forgeplan_search` to find the correct ID).

Example response shape:

```json
{
  "id": "PRD-042",
  "kind": "prd",
  "status": "active",
  "depth": "standard",
  "title": "Authentication system",
  "tags": ["auth", "security"],
  "updated_at": "2026-04-11T09:31:00Z",
  "body": "# PRD-042: Authentication system\n\n## Problem\nUsers currently..."
}
```

## Example invocation

```json
{ "id": "PRD-001" }
```

With typical agent context:

> User asks "remind me what PRD-042 says about token expiry". Agent fetches the full body to quote the relevant section.

```json
{ "id": "PRD-042" }
```

## Typical sequence

`forgeplan_list` or `forgeplan_search` returns an ID → `forgeplan_get` pulls the body → agent quotes / reasons / updates. For bulk reads, agents should prefer `forgeplan_search` which returns ranked snippets rather than hitting `get` in a loop.

## CLI equivalent

- [`forgeplan show`](/docs/cli/get/) — human-readable terminal render

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_list`](/docs/mcp/forgeplan_list/) — find the ID first
- [`forgeplan_update`](/docs/mcp/forgeplan_update/) — edit after reading
- [`forgeplan_search`](/docs/mcp/forgeplan_search/) — content-level discovery
