---
title: forgeplan_discover_complete
description: "Complete a brownfield discovery session and return stored finding counts and artifact IDs. Health checks and proposal synthesis are separate agent steps."
---

Completes a session started by `forgeplan_discover_start`. The selected implementation loads the saved session, marks it completed, saves it, and returns finding counts and created artifact IDs. It does **not** run health or generate proposals, despite the runtime description reproduced below.

**Category**: Brownfield Discovery

## When an agent calls this

- After the seven protocol phases have been walked and all findings emitted.
- When the user says "wrap up discovery" / "propose next steps" / "finish the scan".
- Before planning follow-up work: inspect the recorded findings and prepare proposals yourself.

## What it does

1. Loads the session and its stored findings.
2. Sets `session_status` to `completed` and saves the session.
3. Returns `total_findings`, `phase_counts`, `tier_counts`, `artifacts_created`, and `completed_at`.

## Health and proposal synthesis are separate

Call `forgeplan_health` separately if needed. The agent can then review findings and propose PROBs for risks, PRDs for user needs, or RFCs for implementation choices; create only approved, fully authored artifacts through `forgeplan_new`.

Completion does not activate the finding artifacts. The current finding handler also does not reject a session merely because it is marked completed, so treat completion as a workflow marker, not a write barrier.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `session_id` | `string` | yes | Existing session handle from `forgeplan_discover_start`. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::DiscoverCompleteParams`_

## Returns

A summary of the saved session. Illustrative excerpt:

```json
{
  "session_id": "discover-legacy-billing-service-2026-04-11T10:15:00Z",
  "session_status": "completed",
  "total_findings": 2,
  "phase_counts": { "code": 1, "docs": 1 },
  "tier_counts": { "1": 1, "3": 1 },
  "artifacts_created": ["NOTE-087", "PROB-020"]
}
```

The legacy `status` field also refers to the session, not the artifacts. Read `session_status` and review each finding's own lifecycle status.

## Example invocation

```json
{ "session_id": "discover-legacy-billing-service-2026-04-11T10:15:00Z" }
```

## Typical sequence

```
discover_start → …many discover_finding calls… → discover_complete
                                                    ↓ agent reviews findings; optional separate health call
                                                 forgeplan_new (approved follow-up artifacts)
                                                    ↓
                                                 forgeplan_validate / forgeplan_reason …
```

## CLI equivalent

- [`forgeplan discover complete`](/docs/cli/discover-complete/) - same finalisation from the terminal.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_discover_start`](/docs/mcp/forgeplan_discover_start/) - kick off a session
- [`forgeplan_discover_finding`](/docs/mcp/forgeplan_discover_finding/) - report observations
- [`forgeplan_health`](/docs/mcp/forgeplan_health/) - a separate post-discovery health call
- [`forgeplan_new`](/docs/mcp/forgeplan_new/) - promote a proposal into a real artifact

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Complete a discovery session. Generates a summary report with findings per phase/tier, runs forgeplan health, and marks the session as completed.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `session_id` | yes | string | Session ID to complete |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "session_id": {
      "description": "Session ID to complete",
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
    "session_id"
  ],
  "title": "DiscoverCompleteParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
