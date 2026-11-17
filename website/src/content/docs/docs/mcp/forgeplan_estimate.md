---
title: forgeplan_estimate
description: "Estimate effort for an artifact based on FR and Phase items. Returns multi-grade breakdown (Junior/Middle/Senior/Principal/AI) with confidence scoring."
---

Estimate effort (in hours) for an artifact by parsing its Functional Requirements and Implementation Phase checkboxes, then applying a multi-grade profile (Junior / Middle / Senior / Principal / AI). Returns a confidence-scored breakdown per grade, optionally using LLM-based complexity scoring or manual overrides.

**Category**: Quality

## When an agent calls it

- **Sprint planning**: get hours for each PRD / RFC before committing scope.
- **Grade matching**: see how many hours the task takes for your own grade via `my_grade`.
- **AI-assisted plans**: highlight AI hours, calculated using task-type multipliers plus review overhead, not a universal speedup factor.
- **Calibration feedback loop**: compare actual vs estimated after the sprint to tune the profile.

## Input parameters

| Name | JSON type | Required | Description / constraints |
|---|---|---|---|
| `complexity` | `string \| null` | no | Manual complexity overrides: "FR-001=5,FR-002=3" Schema default: `null`. |
| `grade` | `string \| null` | no | Override grade for all items string: Enum: `"junior"`, `"middle"`, `"senior"`, `"principal"`, `"ai"`. |
| `id` | `string` | yes | Artifact ID to estimate |
| `llm_score` | `boolean \| null` | no | Use LLM-based complexity scoring instead of rule-based heuristics Schema default: `null`. |
| `my_grade` | `boolean \| null` | no | Auto-detect grade from config grade_profile + artifact domain inference Schema default: `null`. |
| `workspace` | `string \| null` | no | Optional explicit workspace directory for this call. When set, the MCP server resolves the `.forgeplan/` projection from this path instead of the server's startup CWD. Use this when the calling agent is running in a git worktree separate from the MCP server's launch directory. PRD-078 / ADR-015. Accepts an absolute or tilde-expanded path. Schema default: `null`. |

Required means the key must be present. `null` is a permitted value only when listed in the JSON type. Schema defaults are not necessarily the handler defaults described in the text.

_Schema source: MCP `tools/list`, Forgeplan 0.38.0, `inputSchema`._

## Returns

Success is JSON in MCP text content. Top-level fields are `artifact_id`, `artifact_title`, `items`, `totals` (grade → numeric hours), `total_score`, `confidence`, `confidence_reasons`, optional nonempty `hints`, and `_next_action`. There is no `by_grade`, `breakdown` or `total_items`.

Each item has `id`, `description`, `complexity` (serialized enum name such as `"Medium"`, not the Fibonacci number), `task_type` (for example `"pure_coding"`), `hours` (grade → hours), and `score`. Confidence is one overall score, not a per-grade score. Hints contain `level`, `message` and optional `action`.

`grade` only adds `highlighted_grade`; all grades are still calculated. Without explicit `grade`, `my_grade: true` adds `highlighted_grade` and `inferred_domain` from the configured profile. Manual complexity overrides use values `1,2,3,5,8,13` and take priority over heuristic/LLM scoring; malformed overrides return an MCP error, and unmatched item IDs have no effect.

`llm_score` defaults to false. If requested without LLM configuration or if provider scoring fails, scoring falls back to heuristics; the response does not certify that an LLM ran. No FR/Phase items yields a successful empty estimate (`items: []`, `totals: {}`, score/confidence zero), without grade highlighting. The tool does not save the estimate or tune configuration automatically; a missing artifact returns an MCP error.

Illustrative item excerpt for Medium pure coding under default multipliers:

```json
{
  "id": "FR-001",
  "description": "Implement the parser",
  "complexity": "Medium",
  "task_type": "pure_coding",
  "hours": {"junior": 16.0, "middle": 12.0, "senior": 8.0, "principal": 5.6, "ai": 1.04},
  "score": 24.0
}
```

These are model estimates, not observed completion times. AI hours include configured review overhead; coordination tasks do not inherit the pure-coding multiplier.

## Example invocation

Arguments for this tool (not a JSON-RPC envelope). Replace illustrative IDs, session handles and paths with values from your workspace. Schema validity does not bypass workspace, lifecycle or consent checks.

```json
{
  "id": "PRD-001",
  "grade": "senior"
}
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

- [`forgeplan_calibrate`](/docs/mcp/forgeplan_calibrate/) - separate depth suggestion; not an automatic estimation input.
- [`forgeplan_progress`](/docs/mcp/forgeplan_progress/) - actual completion % vs estimate.
- [Methodology guide](/docs/methodology/overview/)
