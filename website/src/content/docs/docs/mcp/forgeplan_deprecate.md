---
title: forgeplan_deprecate
description: "Deprecate an artifact (active → deprecated) with a reason."
---

Moves an artifact to the terminal `deprecated` status without a replacement. Unlike `supersede` (which implies "use this new thing instead"), `deprecate` means "we're not doing this any more". The required `reason` is stored on the artifact and surfaced in every future `forgeplan_get` so future readers understand why it was retired.

**Category**: Lifecycle

## When an agent calls this

- Feature retirement: "we're removing rate limiting from v3, deprecate PRD-020".
- Abandoned direction: an active or stale exploratory PRD that turned out infeasible.
- Cleanup during `forgeplan_health` remediation: stale artifacts that nobody will renew.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to deprecate |
| `reason` | `string` | yes | Reason for deprecation |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The confirmation contains `deprecated`, `reason`, `dependents_affected`, `receipt_id`, `resolved_workspace`, `resolved_via` and `_next_action`. `dependents_affected` lists incoming references, not automatically migrated artifacts or notifications.

The reason is appended to the body's `## Deprecation` section with a date, not stored as a new frontmatter field.

## Guards and side effects

- Only `active` or `stale` → `deprecated` is accepted. A draft, already deprecated or superseded artifact is refused; schema validity alone does not satisfy this gate.
- The handler locks the workspace, captures a receipt before mutation and syncs the projection. Receipt-write failure aborts retirement. A later gate failure may leave a pre-written unused receipt; do not assume it records a completed transition.
- Success changes status and appends the reason. Projection rendering, advisory phase advancement to `done`, and dependent rescoring for displaced evidence are best-effort; a logged failure does not necessarily fail the call.
- The MCP tool does not prompt for confirmation. Retire only the artifact the user/workflow authorized, and review affected dependents. A valid receipt permits recovery, but recovery is a real mutation with the [0.38.0 restore limitations](/docs/mcp/forgeplan_restore/).

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "PRD-001",
  "reason": "Feature cancelled; no replacement."
}
```

## Typical sequence

Inspect the target with `forgeplan_get` or `forgeplan_list` → confirm retirement → `forgeplan_deprecate` with `id` and `reason` → inspect affected dependents → `forgeplan_health`. For expired-but-still-valid decisions, the CLI [`forgeplan renew`](/docs/cli/renew/) extends validity instead; it is not an MCP tool.

## CLI equivalent

- [`forgeplan deprecate`](/docs/cli/deprecate/) - same operation

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) - when a replacement exists
- [`forgeplan_stale`](/docs/mcp/forgeplan_stale/) - precedes deprecate for expired artifacts
- [`forgeplan_delete`](/docs/mcp/forgeplan_delete/) - soft-delete with receipt-based recovery
