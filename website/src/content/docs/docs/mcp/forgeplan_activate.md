---
title: forgeplan_activate
description: "Activate draft → active, subject to lifecycle checks and the explicit force override."
---

Promotes an artifact from `draft` to `active`. Without `force`, MUST validation failures block activation; `force: true` bypasses validation and the configured evidence-provenance gate, so it is not proof of quality. Successful activation updates canonical status and may advance the advisory phase to `done`. List active artifacts with `forgeplan_list {"status":"active"}`. Workflow requirements for implementation and evidence remain separate from the tool's checks.

**Category**: Lifecycle

## When an agent calls this

- After filling the PRD stub + validate PASS + evidence attached + R_eff > 0 - "ready to ship".
- At the end of a sprint checklist: all FRs checked, all tests green, PR merged - time to flip the switch.
- When the user says "mark PRD-042 as active now" and the agent has verified the gates.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to activate. |
| `force` | `bool` | no (default: `false`) | Force activation even if validation has MUST errors. |
| `workspace` | `string \| null` | no | Explicit absolute or tilde-expanded workspace path; otherwise use the server's workspace resolution chain. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ActivateParams`_

## Returns

The JSON text in MCP `content[]` contains `artifact_id`, `forced`,
`must_errors`, `message`, and `_next_action`, plus workspace-resolution and
index-publication fields. There is no `activated_at`, `from`, or `to` field.
On a migrated workspace, inspect `index_pending`: a canonical status write
can succeed before derived index publication is acknowledged.

Failures are MCP tool errors containing explanatory text, not the old
`{"ok":false,"error":"validation_failed"}` envelope. Read the error before
deciding whether validation, lifecycle state, provenance, or workspace
resolution needs correction.

## Example invocation

```json
{ "id": "PRD-001" }
```

Illustrative agent context (not a recorded test or score):

> All FRs implemented, tests green, evidence linked, R_eff = 0.87. Agent activates.

```json
{ "id": "PRD-042" }
```

## Typical sequence

Full cycle: `forgeplan_new` → `forgeplan_update` → `forgeplan_validate` PASS → code → `forgeplan_new evidence` → `forgeplan_link` → `forgeplan_score` > 0 → `forgeplan_activate`. Never activate a PRD without code and evidence - it becomes a false promise.

## CLI equivalent

- [`forgeplan activate`](/docs/cli/activate/) - same transition

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) - pre-flight check
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) - next lifecycle stage
- [`forgeplan_deprecate`](/docs/mcp/forgeplan_deprecate/) - terminal retirement

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Activate an artifact (draft → active). Requires all MUST validation rules to pass.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `force` | no | boolean | Force activation even if validation has MUST errors |
| `id` | yes | string | Artifact ID to activate |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "force": {
      "default": false,
      "description": "Force activation even if validation has MUST errors",
      "type": "boolean"
    },
    "id": {
      "description": "Artifact ID to activate",
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
  "title": "ActivateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
