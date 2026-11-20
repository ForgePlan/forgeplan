---
title: forgeplan_update
description: "Update artifact metadata (status, title) and/or body. Re-renders markdown projection after update."
---

Patches an existing artifact's body and/or frontmatter metadata in-place. This is the direct-edit path - the agent uses it to fill a freshly created stub, rewrite a section after user feedback, or rename an artifact. For lifecycle transitions (draft → active → superseded) the agent must use the dedicated `forgeplan_activate` / `forgeplan_supersede` / `forgeplan_deprecate` tools instead, which enforce validation gates.

**Category**: Editing Artifacts

## When an agent calls this

- Immediately after `forgeplan_new` to populate MUST sections (Problem, Goals, Non-Goals, FR).
- Applying a user-requested edit: "add a section on observability to RFC-007".
- Fixing a validator finding: `forgeplan_validate` reports a missing section, agent writes it and re-validates.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to update. |
| `status` | `string` or `null` | no | The schema enum lists `draft`, `active`, `superseded`, `deprecated`, but the handler permits only a direct `draft` write. Use lifecycle tools for the other values. |
| `title` | `string` | no | New title. |
| `body` | `string` | no | New body content (full markdown replacement). |

_Schema source: `crates/forgeplan-mcp/src/server.rs::UpdateParams`_

## Returns

Returns `id`, `message`, `status`, and `title`, with workspace trace and reconciliation information. Canonical Markdown is updated first; `index_pending` reports derived-index reconciliation separately. `active`, `superseded`, and `deprecated` status writes are rejected even though the schema advertises those enum values. Omitting all update fields, or passing them all as `null`, produces “Nothing to update.”

Illustrative response excerpt:

```json
{
  "id": "PRD-042",
  "message": "Updated successfully",
  "status": "draft",
  "title": "Authentication and session system"
}
```

## Example invocation

```json
{ "id": "PRD-042", "title": "Authentication and session system" }
```

With typical agent context:

> Agent just ran `forgeplan_new`, got `PRD-042`, and now fills the MUST sections in a single body replacement.

```json
{ "id": "PRD-042", "body": "# PRD-042: Authentication system\n\n## Problem\n..." }
```

## Typical sequence

`forgeplan_new` → `forgeplan_update` (fill stub) → `forgeplan_validate` → loop on any findings → `forgeplan_activate`. For late edits to an active artifact, the pattern is `forgeplan_get` → compute patch → `forgeplan_update`.

## CLI equivalent

- [`forgeplan update`](/docs/cli/update/) - for example, `forgeplan update PRD-042 --title "Authentication and session system"` or `forgeplan update PRD-042 --body @body.md`. The CLI also exposes `--depth`; the MCP schema does not.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - the correct path for `draft → active`
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - verify after the edit
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) - replace instead of edit

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Update artifact metadata (status, title) and/or body. Re-renders markdown projection after update.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `body` | no | string \| null | New body content |
| `id` | yes | string | Artifact ID to update |
| `status` | no | StatusKind \| null | New status |
| `title` | no | string \| null | New title |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "StatusKind": {
      "enum": [
        "draft",
        "active",
        "superseded",
        "deprecated"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "body": {
      "default": null,
      "description": "New body content",
      "type": [
        "string",
        "null"
      ]
    },
    "id": {
      "description": "Artifact ID to update",
      "type": "string"
    },
    "status": {
      "anyOf": [
        {
          "$ref": "#/$defs/StatusKind"
        },
        {
          "type": "null"
        }
      ],
      "description": "New status"
    },
    "title": {
      "default": null,
      "description": "New title",
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
  "required": [
    "id"
  ],
  "title": "UpdateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
