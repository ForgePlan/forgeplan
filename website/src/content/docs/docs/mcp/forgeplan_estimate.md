---
title: forgeplan_estimate
description: "Estimate effort for an artifact based on FR and Phase items. Returns multi-grade breakdown (Junior/Middle/Senior/Principal/AI) with confidence scoring."
---

Estimate effort (in hours) for an artifact by parsing its Functional Requirements and Implementation Phase checkboxes, then applying a multi-grade profile (Junior / Middle / Senior / Principal / AI). Returns a confidence-scored breakdown per grade, optionally using LLM-based complexity scoring or manual overrides.

**Category**: Quality

## When an agent calls it

- **Sprint planning** — get hours for each PRD / RFC before committing scope.
- **Grade matching** — see how many hours the task takes for your own grade via `my_grade`.
- **AI-assisted plans** — `grade: "ai"` applies AI multipliers (typically 0.03–0.4× of senior) for tasks that can be delegated.
- **Calibration feedback loop** — compare actual vs estimated after the sprint to tune the profile.

## Input parameters

| Name | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | yes | Artifact ID to estimate. |
| `grade` | `string` | no | Override grade for all items: `junior`, `middle`, `senior`, `principal`, `ai`. |
| `my_grade` | `bool` | no | Auto-detect grade from config `grade_profile` + artifact domain inference. |
| `llm_score` | `bool` | no | Use LLM-based complexity scoring instead of rule-based heuristics. |
| `complexity` | `string` | no | Manual complexity overrides, e.g. `"FR-001=5,FR-002=3"`. |

_Schema source: `crates/forgeplan-mcp/src/server.rs::EstimateParams`_

## Returns

```json
{
  "id": "PRD-039",
  "total_items": 12,
  "by_grade": {
    "junior": { "hours": 96, "confidence": 0.6 },
    "middle": { "hours": 52, "confidence": 0.75 },
    "senior": { "hours": 28, "confidence": 0.85 },
    "principal": { "hours": 18, "confidence": 0.8 },
    "ai": { "hours": 4.2, "confidence": 0.5 }
  },
  "breakdown": [
    { "item": "FR-001", "complexity": 3, "senior_hours": 4 }
  ]
}
```

## Example invocation

```json
{ "id": "PRD-039", "my_grade": true }
```

## Typical sequence

1. `forgeplan_estimate` with `my_grade: true` — get personal hours.
2. Compare with sprint capacity — scope down if needed.
3. After sprint: feed actual vs estimate back into `grade_profile` config.

## CLI equivalent

```bash
forgeplan estimate PRD-039 --my-grade
forgeplan estimate PRD-039 --grade ai
```

## See also

- [`forgeplan_calibrate`](/docs/mcp/forgeplan_calibrate/) — depth suggestion (feeds into estimation).
- [`forgeplan_progress`](/docs/mcp/forgeplan_progress/) — actual completion % vs estimate.
- [Methodology guide](/docs/methodology/overview/)
