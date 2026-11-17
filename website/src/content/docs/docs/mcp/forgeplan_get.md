---
title: forgeplan_get
description: "Read a full artifact by ID. Returns all metadata and body content."
---

Returns a single artifact's full markdown body plus frontmatter metadata. This is the canonical "read" operation - the agent calls it when it needs the actual contents of a PRD/RFC/ADR to reason about, quote, or update. Unlike `forgeplan_list`, the response contains everything the agent needs to understand the artifact.

**Category**: Reading Artifacts

## When an agent calls this

- User asks "what does PRD-042 say about rate limits?" - agent fetches then quotes the relevant section.
- Before calling `forgeplan_update` - the agent needs the current body to produce a diff-aware patch.
- Before `forgeplan_reason` - agent pre-reads so it can describe the context to the user if the LLM call is slow.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to read |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON encoded in MCP `content[0].text`, not a `structuredContent` object. The fixed record fields are `id`, `kind`, `status`, `title`, `body`, `depth`, `author`, `parent_epic`, `r_eff_score`, `valid_until`, `created_at`, `updated_at`, `id_canonical`, `id_display`, and `links`. `author`, `parent_epic`, and `valid_until` may be null. Optional identity fields `slug`, `predicted_number`, and `assigned_number` are omitted when absent. There is no top-level `tags` field; arbitrary frontmatter remains in `body`.

`links.outbound` contains `{target, relation}` objects; `links.inbound` contains `{source, relation}` objects. A failed relation lookup degrades to an empty array. `_next_action` is a status-dependent hint and may mention an advisory phase or live claim; it is not an automatic action or write lock. Both display IDs and canonical slugs are resolved. A missing artifact returns an MCP tool error (`isError: true`) with plain text, not this JSON shape.

Illustrative response (values are not an observed workspace result):

```json
{
  "id": "PRD-042",
  "kind": "prd",
  "status": "active",
  "depth": "standard",
  "title": "Authentication system",
  "author": null,
  "parent_epic": null,
  "r_eff_score": 0.0,
  "valid_until": null,
  "created_at": "2026-04-10T09:31:00Z",
  "updated_at": "2026-04-11T09:31:00Z",
  "id_canonical": "prd-042",
  "id_display": "PRD-042",
  "links": {"outbound": [], "inbound": []},
  "_next_action": "Active. Verify trust: `forgeplan_score prd-042`.",
  "body": "# PRD-042: Authentication system\n\n## Problem\nUsers currently..."
}
```

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "PRD-001"
}
```

## Typical sequence

`forgeplan_list` or `forgeplan_search` returns an ID → `forgeplan_get` pulls the body → agent quotes, reasons, or updates. Search discovers candidates; smart-search results do not contain body snippets, so use `get` for the actual text.

## CLI equivalent

- [`forgeplan get`](/docs/cli/get/) - human-readable terminal render

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_list`](/docs/mcp/forgeplan_list/) - find the ID first
- [`forgeplan_update`](/docs/mcp/forgeplan_update/) - edit after reading
- [`forgeplan_search`](/docs/mcp/forgeplan_search/) - content-level discovery
