---
title: forgeplan_estimate
description: "Estimate effort for an artifact based on FR and Phase items. Returns multi-grade breakdown (Junior/Middle/Senior/Principal/AI) with confidence scoring."
---

Estimate effort (in hours) for an artifact by parsing its Functional Requirements and Implementation Phase checkboxes, then applying a multi-grade profile (Junior / Middle / Senior / Principal / AI). Returns a confidence-scored breakdown per grade, optionally using LLM-based complexity scoring or manual overrides.

**Category**: Quality

## When an agent calls it

- **Sprint planning** - get hours for each PRD / RFC before committing scope.
- **Grade matching** - see how many hours the task takes for your own grade via `my_grade`.
- **AI-assisted plans** - `grade: "ai"` highlights the AI grade; it does not switch the engine to a different calculation. Default AI grade and task-type multipliers are configurable model assumptions, not a measured productivity guarantee.
- **Calibration feedback loop** - compare actual vs estimated after the sprint to tune the profile.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to estimate. |
| `grade` | `string` | no | Highlight one grade in the result: `junior`, `middle`, `senior`, `principal`, `ai`. All grade totals are still calculated. |
| `my_grade` | `bool` | no | Auto-detect grade from config `grade_profile` + artifact domain inference. |
| `llm_score` | `bool` | no | Use LLM-based complexity scoring instead of rule-based heuristics. |
| `complexity` | `string` | no | Manual complexity overrides, e.g. `"FR-001=5,FR-002=3"`. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::EstimateParams`_

## Returns

Illustrative empty-items response excerpt, not a measured estimate. Nonempty results use `items`, `totals`, `total_score`, `confidence`, and `confidence_reasons`; confidence is report-wide, not per-grade. `llm_score: true` falls back to heuristic scoring when no LLM is configured. The handler caps the `complexity` string at 4096 bytes, although that cap is absent from the schema.

```json
{
  "artifact_id": "PRD-039",
  "artifact_title": "Authentication system",
  "items": [],
  "totals": {},
  "total_score": 0.0,
  "confidence": 0.0,
  "confidence_reasons": []
}
```

## Example invocation

```json
{ "id": "PRD-039", "my_grade": true }
```

## Typical sequence

1. `forgeplan_estimate` with `my_grade: true` - get personal hours.
2. Compare with sprint capacity - scope down if needed.
3. After sprint: feed actual vs estimate back into `grade_profile` config.

## CLI equivalent

```bash
forgeplan estimate PRD-039 --my-grade
forgeplan estimate PRD-039 --grade ai
```

## See also

- [`forgeplan_calibrate`](/docs/mcp/forgeplan_calibrate/) - depth suggestion (feeds into estimation).
- [`forgeplan_progress`](/docs/mcp/forgeplan_progress/) - actual completion % vs estimate.
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured tools/list

Source: `forgeplan 0.39.0+constellation.s1.1`, `tools/list`, profile `full`.

Estimate effort for an artifact based on FR and Phase items. Returns multi-grade breakdown (Junior/Middle/Senior/Principal/AI) with confidence scoring.

### Input parameters

| Name | Required | Type | Description |
|---|---|---|---|
| `complexity` | no | string \| null | Manual complexity overrides: "FR-001=5,FR-002=3" |
| `grade` | no | GradeKind \| null | Override grade for all items |
| `id` | yes | string | Artifact ID to estimate |
| `llm_score` | no | boolean \| null | Use LLM-based complexity scoring instead of rule-based heuristics |
| `my_grade` | no | boolean \| null | Auto-detect grade from config grade_profile + artifact domain inference |
| `workspace` | no | string \| null | Optional explicit workspace directory for this call. When set, the MCP<br>server resolves the `.forgeplan/` projection from this path instead of<br>the server's startup CWD. Use this when the calling agent is running<br>in a git worktree separate from the MCP server's launch directory.<br>PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. |

<details>
<summary>JSON Schema</summary>

```json
{
  "$defs": {
    "GradeKind": {
      "enum": [
        "junior",
        "middle",
        "senior",
        "principal",
        "ai"
      ],
      "type": "string"
    }
  },
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "complexity": {
      "default": null,
      "description": "Manual complexity overrides: \"FR-001=5,FR-002=3\"",
      "type": [
        "string",
        "null"
      ]
    },
    "grade": {
      "anyOf": [
        {
          "$ref": "#/$defs/GradeKind"
        },
        {
          "type": "null"
        }
      ],
      "description": "Override grade for all items"
    },
    "id": {
      "description": "Artifact ID to estimate",
      "type": "string"
    },
    "llm_score": {
      "default": null,
      "description": "Use LLM-based complexity scoring instead of rule-based heuristics",
      "type": [
        "boolean",
        "null"
      ]
    },
    "my_grade": {
      "default": null,
      "description": "Auto-detect grade from config grade_profile + artifact domain inference",
      "type": [
        "boolean",
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
  "title": "EstimateParams",
  "type": "object"
}
```

</details>
<!-- forgeplan-runtime:end -->
