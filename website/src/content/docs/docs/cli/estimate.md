---
title: forgeplan estimate
description: "Calibrated effort estimate from FR/Phase items with grade profiles and AI multipliers."
---

`forgeplan estimate` extracts FR and phase work items, assigns Fibonacci complexity,
and maps that complexity to base senior hours. It computes all grade totals;
`--grade` and `--my-grade` select the highlighted grade rather than changing every item.
Human grades use configured grade multipliers. AI estimates use task-type multipliers
and review overhead. The calculator does not currently apply `safety_margin`.
These are planning heuristics, not measured delivery commitments.

## When to use

- Sprint planning: estimate every candidate PRD/RFC to fit capacity (≈40–50% of nominal).
- Capacity check before committing to an epic - is this 2 weeks or 2 months?
- Delegating to a teammate - `--grade middle` to see what the task looks like for them.
- Agent planning - `--grade ai` highlights AI estimates; default task multipliers range from 0.10 to 1.00 before review overhead.

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
      --grade <GRADE>            Highlight one grade total (junior|middle|senior|principal|ai)
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

The report lists work items, complexity, task type, grade totals and confidence.
With default settings, a complexity-3 item has 8 base senior hours. A pure-coding
AI estimate is `8 × 0.10 × 1.30 = 1.04` hours. These are arithmetic examples,
not benchmark results; configured multipliers can change them.

### Compare human vs AI-assisted path

```bash
forgeplan estimate PRD-001 --grade senior
forgeplan estimate PRD-001 --grade ai
```

AI task multipliers default to 0.10 (coding), 0.25 (coding/infra), 0.30
(design/coding), 0.50 (infra) and 1.00 (coordination), with 30% review overhead.

### Manual complexity override

```bash
forgeplan estimate PRD-001 --complexity FR-001=8,FR-002=2
```

Use when heuristics misjudge complexity (e.g., an "easy" FR hides a migration).

### LLM-scored estimation

```bash
forgeplan estimate PRD-001 --llm-score
```

Asks the configured LLM to score complexity per item, with rule-based fallback
on model failure. Review the classifications rather than assuming better accuracy.

## Output interpretation

| Line         | What it shows                                                      |
|--------------|--------------------------------------------------------------------|
| per-item row | complexity mapped to base senior hours, then grade/task factors |
| Totals | sum of computed item hours for each grade |
| Review | configurable AI-only overhead; default 30% |
| Safety | config field exists, but the current calculator does not add it |
| Confidence | heuristic input completeness, not a measured error bound |

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

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan estimate --help`.

```text
Estimate effort for an artifact based on FR and Phase items

Usage: forgeplan estimate [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID to estimate

Options:
      --grade <GRADE>            Override grade for all items (junior|middle|senior|principal|ai)
      --my-grade                 Use grade profile from config (domain-aware)
      --llm-score                Use LLM-based complexity scoring instead of rule-based heuristics
      --complexity <COMPLEXITY>  Manual complexity overrides: FR-001=5,FR-002=3 (Fibonacci: 1,2,3,5,8,13)
      --json                     Output as JSON for machine consumption
  -h, --help                     Print help
  -V, --version                  Print version
```
<!-- forgeplan-runtime:end -->
