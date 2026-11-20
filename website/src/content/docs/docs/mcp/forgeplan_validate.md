---
title: forgeplan_validate
description: "Validate artifact completeness against schema rules. Checks required sections per artifact kind and depth level. Returns structured findings with severity (MUST/SHOULD/COULD)."
---

Runs kind- and depth-aware rules for required sections, frontmatter, density and cross-references. Zero MUST findings passes this validation step; it does not establish implementation, evidence strength, activation eligibility, or satisfaction of external project policy.

**Category**: Quality & Validation

## When an agent calls this

- Immediately after filling in a stub - "did I cover everything the template needs?"
- As a loop with `forgeplan_update`: validate → fix finding → validate → repeat until PASS.
- Pre-activation check: if any MUST remains, `forgeplan_activate` will fail anyway, so validate first.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | no | Artifact ID to validate. Validates all artifacts if omitted. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::ValidateParams`_

## Returns

Returns aggregate `total_artifacts`, `total_passed`, `total_errors`, `total_warnings`, and per-artifact `results`. Each result has `artifact_id`, `kind`, `depth`, `passed`, `error_count`, `warning_count`, and `findings`; findings use `rule_id`, `severity`, `message`, and optional `section`.

`passed: true` means zero MUST findings, not authorization to activate. Review lifecycle evidence/stub gates and applicable policy separately.

Illustrative response excerpt:

```json
{
  "total_artifacts": 1,
  "total_passed": 1,
  "total_errors": 0,
  "total_warnings": 0,
  "results": [
    { "artifact_id": "RFC-016", "kind": "Rfc", "depth": "Standard", "passed": true, "error_count": 0, "warning_count": 0, "findings": [] }
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

`forgeplan_update` → `forgeplan_validate` → fix findings → revalidate → implement and record actual evidence → review lifecycle and policy requirements before activation. A deliberately retained draft is not a validation failure.

## CLI equivalent

- [`forgeplan validate`](/docs/cli/validate/) - CLI validation uses the same rule domain. Recorded compatibility scenarios do not certify every validation path; see [Constellation](/docs/reference/constellation/) for coverage and remaining limits.

## See also

- [MCP overview](/docs/mcp/)
- [`forgeplan_activate`](/docs/mcp/forgeplan_activate/) - the gate validate enforces
- [`forgeplan_score`](/docs/mcp/forgeplan_score/) - orthogonal quality signal (R_eff)
- [`forgeplan_review`](/docs/mcp/forgeplan_review/) - human-readable readiness report

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Validate artifact completeness against schema rules. Checks required sections per artifact kind and depth level. Returns structured findings with severity (MUST/SHOULD/COULD).

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `id` | no | string \| null | Artifact ID to validate (validates all if omitted) |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "id": {
      "default": null,
      "description": "Artifact ID to validate (validates all if omitted)",
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
  "title": "ValidateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
