---
title: forgeplan_validate
description: "Validate artifact completeness against schema rules. Checks required sections per artifact kind and depth level. Returns structured findings with severity (MUST/SHOULD/COULD)."
---

Runs the rule-based validator over an artifact. Each kind (PRD, RFC, ADR, Epic, Spec) has a depth-aware ruleset (30+ rules) checking for required sections, frontmatter fields, density, and cross-references. MUST failures block `forgeplan_activate`. This is the agent's primary quality gate before declaring an artifact "done".

**Category**: Quality & Validation

## When an agent calls this

- Immediately after filling in a stub - "did I cover everything the template needs?"
- As a loop with `forgeplan_update`: validate → fix finding → validate → repeat until PASS.
- Pre-activation check: if any MUST remains, `forgeplan_activate` will fail anyway, so validate first.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `id` | `string \| null` | no | Artifact ID to validate (validates all if omitted) Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP `content[0].text` with `total_artifacts`, `total_passed`, `total_errors`, `total_warnings`, `results`, and an optional `_next_action`. Each result has `artifact_id`, `kind`, `depth`, `passed`, `error_count`, `warning_count`, and `findings`; each finding has `rule_id`, `severity` (`MUST`, `SHOULD`, `COULD`), `message`, and nullable `section`. There is no `status: PASS` field.

Omit `id` to validate all stored records. A supplied display ID is matched case-insensitively; a missing ID returns a plain-text MCP tool error (`isError: true`). Validation findings are a successful report, even when `passed` is false. A passing validation can advance the advisory phase to `validate` when tracking is enabled, but never activates the artifact. Passing MUST checks alone does not satisfy all activation gates; use `review`.

Illustrative empty-workspace response:

```json
{
  "total_artifacts": 0,
  "total_passed": 0,
  "total_errors": 0,
  "total_warnings": 0,
  "results": [],
  "_next_action": "All passed! Implement → create EvidencePack with structured fields (verdict/congruence_level/evidence_type) → `forgeplan_link EVID-XXX <id>` → `forgeplan_activate <id>`."
}
```

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{}
```

## Typical sequence

`forgeplan_update` → `forgeplan_validate` → (if MUST failures) `forgeplan_update` fix → `forgeplan_validate` again → PASS → `forgeplan_activate`. The same tool is also useful in `forgeplan_health`'s remediation loop when cleaning up blind spots.

## CLI equivalent

- [`forgeplan validate`](/docs/cli/validate/) - same rules, terminal output

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - the gate validate enforces
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - orthogonal quality signal (R_eff)
- [`forgeplan_review`](/docs/mcp/forgeplan_review/) - human-readable readiness report
