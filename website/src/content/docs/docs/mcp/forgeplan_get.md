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

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Display artifact ID (`PRD-042`) or canonical slug (`prd-authentication-system`). |
| `workspace` | `string?` | no | Absolute or `~/`-expanded workspace path. Request → `FORGEPLAN_WORKSPACE` → server default → CWD parent discovery. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::GetParams`_

## Returns

The MCP `CallToolResult` carries JSON text in `content[]`; parse the text
payload. Success is an `ArtifactRecordDto` plus `_next_action`, not an
`artifact` envelope:

| Fields | Shape |
|---|---|
| `id`, `kind`, `status`, `title`, `body`, `depth` | Strings |
| `author`, `parent_epic`, `valid_until` | Nullable strings |
| `created_at`, `updated_at` | Strings; may be empty when captured date metadata is absent |
| `r_eff_score` | Number |
| `slug`, `predicted_number`, `assigned_number` | Optional identity fields |
| `id_canonical`, `id_display` | Canonical/display reference strings |
| `links` | `outbound` entries (`target`, `relation`) and `inbound` entries (`source`, `relation`); arrays can be empty |
| `_next_action` | Workflow guidance |

There is no `tags` field in this DTO. Missing IDs return a tool error.
For migrated workspaces in local `0.39.0+constellation.s1.1`, `body` is the
complete canonical Markdown including frontmatter. This differs from full
MCP `forgeplan_constellation_get` (`key`, pinned `artifact.body` prose) and
read-profile `constellation_get` (`id` plus handle). See the
[Constellation reference](/docs/reference/constellation/).

## Example invocation

```json
{ "id": "PRD-001", "workspace": "/path/to/project" }
```

With typical agent context:

> User asks "remind me what PRD-042 says about token expiry". Agent fetches the full body to quote the relevant section.

```json
{ "id": "PRD-042" }
```

## Typical sequence

`forgeplan_list` or `forgeplan_search` returns an ID → `forgeplan_get` pulls the body → agent quotes / reasons / updates. For bulk reads, agents should prefer `forgeplan_search` which returns ranked snippets rather than hitting `get` in a loop.

## CLI equivalent

- [`forgeplan get`](/docs/cli/get/) - terminal read, `--json` for machine output

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_list`](/docs/mcp/forgeplan_list/) - find the ID first
- [`forgeplan_update`](/docs/mcp/forgeplan_update/) - edit after reading
- [`forgeplan_search`](/docs/mcp/forgeplan_search/) - content-level discovery

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Read a full artifact by ID. Returns all metadata and body content.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | yes | string | Artifact ID to read |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "description": "Artifact ID to read",
      "type": "string"
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
  "required": [
    "id"
  ],
  "title": "GetParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
