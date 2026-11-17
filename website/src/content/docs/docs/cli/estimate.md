---
title: forgeplan estimate
description: "Calibrated effort estimate from FR/Phase items with grade profiles and AI multipliers."
---

`forgeplan estimate` extracts FR and Phase work items and estimates all five
grades. Fibonacci complexity 1/2/3/5/8/13 maps to base Senior hours
3/5/8/13/21/34, then configured multipliers apply. This is a rule-based planning
model, not a measured delivery forecast.

## When to use

- Sprint planning: estimate every candidate PRD/RFC to fit capacity (≈40–50% of nominal).
- Capacity check before committing to an epic - is this 2 weeks or 2 months?
- Delegating to a teammate - `--grade middle` to see what the task looks like for them.
- Agent planning - `--grade ai` highlights the AI column; task-type multipliers and human review overhead determine its hours.

## When NOT to use

- On Notes or Problems - they have no FR/Phase structure to estimate.
- As a contract - estimates are for planning, not promises. Calibrate with `calibrate-estimate` after the fact.

## Usage

```text
forgeplan estimate [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID to estimate
```

## Options

```text
      --grade <GRADE>            Override grade for all items (junior|middle|senior|principal|ai)
      --my-grade                 Use grade profile from config (domain-aware)
      --llm-score                Use LLM-based complexity scoring instead of rule-based heuristics
      --complexity <COMPLEXITY>  Manual complexity overrides: FR-001=5,FR-002=3 (Fibonacci: 1,2,3,5,8,13)
      --json                     Output as JSON for machine consumption
  -h, --help                     Print help
  -V, --version                  Print version
```

## Examples

### Estimate a PRD for your own grade profile

```bash
forgeplan estimate PRD-001 --my-grade
```

The text renderer shows a multi-grade table (`ID`, `Description`, `Cmpl`, `Jun`,
`Mid`, `Senior`, `PS`, `AI`), totals, eight-hour-day equivalents, confidence,
and hints. `--grade` / `--my-grade` select the grade named in the heading;
they do not remove other grade columns or change the JSON totals.

Illustrative calculation: a complexity-3 pure-coding item has 8 base Senior
hours. With defaults, Senior is 8h, Middle 12h, and AI
`8 × 0.10 × 1.30 = 1.04h`.

### Compare human vs AI-assisted path

```bash
forgeplan estimate PRD-001 --grade senior
forgeplan estimate PRD-001 --grade ai
```

AI uses task multipliers of 0.10 / 0.25 / 0.30 / 0.50 / 1.00 for pure coding,
coding+infrastructure, design+coding, pure infrastructure, and coordination.
Default review overhead is 30%, applied only to AI hours. These values are configurable.

### Manual complexity override

```bash
forgeplan estimate PRD-001 --complexity FR-001=8,FR-002=2
```

Use when heuristics misjudge complexity (e.g., an "easy" FR hides a migration).

### LLM-scored estimation

```bash
forgeplan estimate PRD-001 --llm-score
```

Requires configured LLM settings. If scoring fails, the scorer falls back to
rules; manual `--complexity` overrides are applied last. The command does not
save generated scores back into the artifact.

## Output interpretation

| Value | Meaning |
|---|---|
| Item hours | Base-hour lookup × human grade multiplier, or AI task multiplier × `(1 + review_overhead)` |
| Totals | Sum of item hours for every grade |
| Score | Fibonacci value × base Senior hours, summed across items |
| Confidence | Structural signal score with reasons, not a probability of delivery |
| Safety margin | Config defaults to 0.50 but the released calculator does not apply it |

`--json` returns `result`, `_next_action`, and outer workflow `hints`.
`result` contains `artifact_id`, `artifact_title`, `items`, `totals`,
`total_score`, `confidence`, `confidence_reasons`, and optional estimation
`hints`. Each item has `id`, `description`, `complexity` (enum string, e.g.
`"Medium"`, not `3`), `task_type` (e.g. `"pure_coding"`), `hours` keyed by
`junior`/`middle`/`senior`/`principal`/`ai`, and `score`. No estimable items
is a successful empty result, not an error.

Red flag: totals over 40h - consider splitting the PRD or escalating depth.

## How it fits the workflow

```
route → new → validate → estimate → sprint commit → code → calibrate-estimate
```

Feed the total into your sprint capacity (a senior dev planning at 50% capacity has
~20h of "estimate hours" per week). After the sprint closes, `calibrate-estimate`
shows how accurate the number was.

## See also

- [`forgeplan calibrate`](/docs/cli/calibrate/) - depth calibration, feeds estimation scale
- [`forgeplan calibrate-estimate`](/docs/cli/calibrate-estimate/) - estimate accuracy after sprint
- [`forgeplan route`](/docs/cli/route/) - routing to depth before estimating
- [Depth Calibration](/docs/methodology/routing/)
- [CLI overview](/docs/cli/)
