---
title: forgeplan_validate
description: "Validate artifact completeness against schema rules. Checks required sections per artifact kind and depth level. Returns structured findings with severity (MUST/SHOULD/COULD)."
---

Runs the rule-based validator over an artifact. Each kind (PRD, RFC, ADR, Epic, Spec) has a depth-aware ruleset (30+ rules) checking for required sections, frontmatter fields, density, and cross-references. MUST failures block `forgeplan_activate`. This is the agent's primary quality gate before declaring an artifact "done".

**Category**: Quality & Validation

## When an agent calls this

- Immediately after filling in a stub — "did I cover everything the template needs?"
- As a loop with `forgeplan_update`: validate → fix finding → validate → repeat until PASS.
- Pre-activation check: if any MUST remains, `forgeplan_activate` will fail anyway, so validate first.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | no | Artifact ID to validate. Validates all artifacts if omitted. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ValidateParams`_

## Returns

A verdict plus a list of findings. Each finding carries a `severity` (MUST / SHOULD / COULD), a `rule_id`, a `message`, and often a `section` locator. `status: PASS` means zero MUST failures — the agent can activate.

Example response shape:

```json
{
  "id": "PRD-042",
  "status": "FAIL",
  "must_count": 1,
  "should_count": 2,
  "findings": [
    { "severity": "MUST", "rule": "prd.has_problem", "message": "Missing ## Problem section" },
    { "severity": "SHOULD", "rule": "prd.density", "message": "Problem section < 50 words" }
  ]
}
```

## Example invocation

```json
{ "id": "PRD-001" }
```

With typical agent context:

> Agent just created PRD-001 and wants to run validation before activation.

```json
{ "id": "PRD-001" }
```

## Typical sequence

`forgeplan_update` → `forgeplan_validate` → (if MUST failures) `forgeplan_update` fix → `forgeplan_validate` again → PASS → `forgeplan_activate`. The same tool is also useful in `forgeplan_health`'s remediation loop when cleaning up blind spots.

## CLI equivalent

- [`forgeplan validate`](/docs/cli/validate/) — same rules, terminal output

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) — the gate validate enforces
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) — orthogonal quality signal (R_eff)
- [`forgeplan_review`](/docs/mcp/forgeplan_review/) — human-readable readiness report
