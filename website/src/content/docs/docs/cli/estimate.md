---
title: forgeplan estimate
description: "Calibrated effort estimate from FR/Phase items with grade profiles and AI multipliers."
---

`forgeplan estimate` turns an artifact's Functional Requirements and Implementation
Phases into an effort number. It reads the complexity of each item (Fibonacci
1/2/3/5/8/13) and multiplies by the grade profile from your config — junior, middle,
senior, principal, or AI — per domain (backend/frontend/devops/ai_ml). Review overhead
and a safety margin are added on top, so the output is a realistic planning number, not
a best-case.

## When to use

- Sprint planning: estimate every candidate PRD/RFC to fit capacity (≈40–50% of nominal).
- Capacity check before committing to an epic — is this 2 weeks or 2 months?
- Delegating to a teammate — `--grade middle` to see what the task looks like for them.
- Agent planning — `--grade ai` to get the AI-assisted fast path (×0.03–0.4 multiplier).

## When NOT to use

- On Notes or Problems — they have no FR/Phase structure to estimate.
- As a contract — estimates are for planning, not promises. Calibrate with `calibrate-estimate` after the fact.

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

Output:

```text
PRD-001 — Auth System
  FR-001  login flow          complexity=5  senior  → 6h
  FR-002  session refresh     complexity=3  senior  → 3h
  FR-003  logout              complexity=2  senior  → 1h
  Subtotal:    10h
  Review:      +20%   → 2h
  Safety:      +15%   → 1.5h
  Total: 13.5h senior  (≈ 2 days)
```

### Compare human vs AI-assisted path

```bash
forgeplan estimate PRD-001 --grade senior
forgeplan estimate PRD-001 --grade ai
```

The AI line uses ×0.03–0.4 multipliers depending on item type; use both numbers to
decide when to drive yourself vs delegate to an agent.

### Manual complexity override

```bash
forgeplan estimate PRD-001 --complexity FR-001=8,FR-002=2
```

Use when heuristics misjudge complexity (e.g., an "easy" FR hides a migration).

### LLM-scored estimation

```bash
forgeplan estimate PRD-001 --llm-score
```

Asks the configured LLM to score complexity per FR. Slower, but catches semantic
nuance the rule engine misses.

## Output interpretation

| Line         | What it shows                                                      |
|--------------|--------------------------------------------------------------------|
| per-FR row   | complexity (Fibonacci) × grade-multiplier = item hours             |
| Subtotal     | sum of item hours                                                  |
| Review       | +20% overhead for review cycles (adversarial, audit)               |
| Safety       | +15% buffer for surprise                                           |
| Total        | realistic sprint-planning number                                   |

Red flag: totals over 40h — consider splitting the PRD or escalating depth.

## How it fits the workflow

```
route → new → validate → estimate → sprint commit → code → calibrate-estimate
```

Feed the total into your sprint capacity (a senior dev planning at 50% capacity has
~20h of "estimate hours" per week). After the sprint closes, `calibrate-estimate`
shows how accurate the number was.

## See also

- [`forgeplan calibrate`](/docs/cli/calibrate/) — depth calibration, feeds estimation scale
- [`forgeplan calibrate-estimate`](/docs/cli/calibrate-estimate/) — estimate accuracy after sprint
- [`forgeplan route`](/docs/cli/route/) — routing to depth before estimating
- [Depth Calibration](/docs/methodology/routing/)
- [CLI overview](/docs/cli/)
