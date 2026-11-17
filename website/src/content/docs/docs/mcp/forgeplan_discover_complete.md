---
title: forgeplan_discover_complete
description: "Complete a discovery session and return finding counts and artifact IDs. Health must be invoked separately."
---

Marks a discovery session as `completed` and returns counts of the findings already recorded through `forgeplan_discover_finding`. In Forgeplan 0.38.0 the handler does **not** run health or generate a proposal set: `_next_action` asks the caller to run `forgeplan_health` separately. Finding artifacts already exist and remain drafts until reviewed and explicitly activated.

**Category**: Brownfield Discovery

## When an agent calls this

- After the seven protocol phases have been walked and all findings emitted.
- When the user says "wrap up discovery" / "propose next steps" / "finish the scan".
- Before starting a Shape → Validate → Code cycle on an accepted finding.

## What it does

1. Loads the session named by `session_id`.
2. Sets its session status to `completed`, records the completion time, and saves it.
3. Counts findings by phase and tier and returns their artifact IDs.
4. Suggests a separate health check and review of the created drafts.

## Review stays with the caller

Completion is not validation or activation. The caller reads the findings, decides which artifacts to keep, fills their required sections and follows the ordinary quality and lifecycle gates. Do not treat `session_status: "completed"` as an artifact status or as proof that the discovered claims are correct.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `session_id` | `string` | yes | Session ID to complete |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

The payload contains `session_id`, `project_name`, `session_status`, `total_findings`, `phase_counts`, `tier_counts`, `artifacts_created`, `completed_at` and `_next_action`. `status` remains a deprecated alias for the session status; `_field_warnings` explains that distinction.

`artifacts_created` names the finding artifacts already created during the session. There is no `health`, `proposed` or `next_steps` result in the 0.38.0 handler.

`phase_counts` maps phase names to counts; `tier_counts` is a JSON object whose
keys are tier numbers rendered as strings. Only observed phases and tiers appear.
`completed_at` is an RFC 3339 timestamp. This is a JSON summary in MCP text content,
not a generated report artifact; there are no workspace trace fields.

The handler does not require all seven phases, findings, or an active session.
An empty session can complete, and repeating completion replaces `completed_at`.
It does not change finding-artifact statuses; a draft may already have been
activated separately, so read current artifacts rather than treating the hint as
a fresh status check. Missing sessions and save failures are plain-text tool errors;
workspace write-resolution may fail at JSON-RPC level.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "session_id": "disc-20261023-120000"
}
```

## Typical sequence

`forgeplan_discover_start` → repeated `forgeplan_discover_finding` calls → `forgeplan_discover_complete` → `forgeplan_health` → review the created drafts → validate and activate the ones to retain.

## CLI equivalent

- [`forgeplan discover complete`](/docs/cli/discover-complete/) - same finalisation from the terminal.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) - kick off a session
- [`forgeplan_discover_finding`](/docs/mcp/forgeplan_discover_finding/) - report observations
- [`forgeplan_health`](/docs/mcp/forgeplan_health/) - separate post-discovery health check
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - create an additional artifact after reviewing the findings
