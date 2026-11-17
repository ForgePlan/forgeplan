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

At least one non-null `title`, `body` or `status` must be supplied. The schema lists all lifecycle status spellings, but the handler permits only a direct reset to `draft`; `active`, `superseded` and `deprecated` must go through the dedicated lifecycle tools. A supplied `body` replaces the full body, not one section; an empty string clears it.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `body` | `string \| null` | no | New body content Schema default: `null`. |
| `id` | `string` | yes | Artifact ID to update |
| `status` | `string \| null` | no | New status string: Enum: `"draft"`, `"active"`, `"superseded"`, `"deprecated"`. |
| `title` | `string \| null` | no | New title Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The success payload contains `id` (the resolved canonical ID), `message: "Updated successfully"`, the resulting `status` and `title`, `resolved_workspace`, `resolved_via` and `_next_action`. It is not a full artifact record and has no `ok`, `changed` or `updated_at` field.

Read the artifact through `forgeplan_get` if the caller needs the resulting body or other metadata. Hints are advisory text, not executable MCP argument syntax.

## Guards and side effects

- IDs and slugs are resolved before the existence check. An unknown artifact or a call containing only `id` (or only null update values) is rejected.
- The handler refuses direct status writes to `active`, `superseded` and `deprecated`; the schema enum does not authorize bypassing lifecycle tools.
- Title validation and configurable title/body byte limits apply. A `body` starting with `@` reads a UTF-8 file on the server machine; a relative path is relative to the server process directory, not the client's filesystem or the `workspace` argument. Parsed YAML frontmatter from that file is removed before storing the body.
- Metadata and body changes use separate projection helpers under the workspace lock. This is not a transactional JSON patch: a later failure can follow an earlier successful metadata write.
- A title change renders the new projection and attempts exact-path cleanup of the old filename. Identity stamping and stale-file cleanup are best-effort. The handler does not run validation or scoring on the updated artifact for you.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "PRD-001",
  "title": "Authentication and sessions"
}
```

## Typical sequence

`forgeplan_new` → `forgeplan_update` (fill stub) → `forgeplan_validate` → loop on any findings → `forgeplan_activate`. For late edits to an active artifact, the pattern is `forgeplan_get` → compute patch → `forgeplan_update`.

## CLI equivalent

- [`forgeplan update`](/docs/cli/update/) - update through the CLI, including `--body @file.md`. Do not edit `.forgeplan/` projections directly; use `forgeplan reindex` to rebuild a derived index after pulling tracked Markdown.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - the correct path for `draft → active`
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - verify after the edit
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) - replace instead of edit
