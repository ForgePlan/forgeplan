---
title: forgeplan_activate
description: "Activate an artifact (draft → active). Requires all MUST validation rules to pass."
---

Promotes an artifact from `draft` to `active`. This is the lifecycle gate that enforces quality: if `forgeplan_validate` would report any MUST failure, activation is rejected with the list of blockers. Once active, the artifact counts in `forgeplan_health`, appears in `forgeplan_list --status active`, and becomes a citable decision. For Notes and Problems there is no validation gate — they activate immediately.

**Category**: Lifecycle

## When an agent calls this

- After filling the PRD stub + validate PASS + evidence attached + R_eff > 0 — "ready to ship".
- At the end of a sprint checklist: all FRs checked, all tests green, PR merged — time to flip the switch.
- When the user says "mark PRD-042 as active now" and the agent has verified the gates.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to activate. |
| `force` | `bool` | no (default: `false`) | Force activation even if validation has MUST errors. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ActivateParams`_

## Returns

The new status plus the transition timestamp. On failure, returns the validator report so the agent can remediate without a second `forgeplan_validate` call.

Example response shape:

```json
{
  "ok": true,
  "id": "PRD-042",
  "from": "draft",
  "to": "active",
  "activated_at": "2026-04-11T10:19:00Z"
}
```

Failure shape:

```json
{
  "ok": false,
  "error": "validation_failed",
  "must_findings": [
    { "rule": "prd.has_problem", "message": "Missing ## Problem section" }
  ]
}
```

## Example invocation

```json
{ "id": "PRD-001" }
```

With typical agent context:

> All FRs implemented, tests green, evidence linked, R_eff = 0.87. Agent activates.

```json
{ "id": "PRD-042" }
```

## Typical sequence

Full cycle: `forgeplan_new` → `forgeplan_update` → `forgeplan_validate` PASS → code → `forgeplan_new evidence` → `forgeplan_link` → `forgeplan_score` > 0 → `forgeplan_activate`. Never activate a PRD without code and evidence — it becomes a false promise.

## CLI equivalent

- [`forgeplan activate`](/docs/cli/activate/) — same transition

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_validate`](/docs/mcp/forgeplan_validate/) — pre-flight check
- [`forgeplan_supersede`](/docs/mcp/forgeplan_supersede/) — next lifecycle stage
- [`forgeplan_deprecate`](/docs/mcp/forgeplan_deprecate/) — terminal retirement
