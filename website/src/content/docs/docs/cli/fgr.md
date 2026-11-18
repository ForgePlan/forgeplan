---
title: forgeplan fgr
description: "Show F-G-R quality scores - Formality, Granularity, Reliability - orthogonal to lifecycle."
---

`forgeplan fgr` prints the **F-G-R** quality triple for an artifact:

- **F — Formality**: fraction of validation checks passed.
- **G — Granularity**: completeness across five content checks — substantive problem/goals/related sections, requirement checkboxes and body length.
- **R — Reliability**: weighted R_eff, relation-count contribution and freshness.

Each axis is in **0–1**. Overall is the **geometric mean** `(F × G × R)^(1/3)`,
not the arithmetic mean. These are ForgePlan heuristics, not current upstream FPF
scales or calibrated probabilities. F-G-R is orthogonal to lifecycle, not permission
to activate.

## When to use

- Quick visual maturity check alongside the detailed validator findings.
- Session start, after `health`, to sort artifacts by "cheapest to level up".
- Comparing two candidate PRDs before committing scope.
- Inspecting the F-G-R inputs before evaluating applicable FPF rules.

## When NOT to use

- As a substitute for `validate` - FGR is a summary, not a rule report.
- On non-decision artifacts where the axes don't apply cleanly (Notes, RefreshReports).

## Usage

```text
forgeplan fgr [OPTIONS] [ID]
```

## Arguments

```text
  [ID]  Artifact ID (scores all if omitted)
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Inspect one PRD

```bash
forgeplan fgr PRD-001
```

The text view has columns `ID`, `Title`, `F`, `G`, `R`, `Grade`.
For illustration, axes `1.0`, `0.8`, `0.7` have an overall of about `0.824`
and grade `A`; this is arithmetic illustration, not a measured artifact.

Grades use strict boundaries: A above 0.8, B above 0.6, C above 0.4,
D above 0.2, otherwise F. No `exploit_fgr` field is emitted by this command.

### Rank all artifacts by maturity

```bash
forgeplan fgr --json | jq '.results | sort_by(.overall) | reverse | .[0:10]'
```

Top 10 most mature artifacts - useful when deciding what to promote or cite.

### Find the cheapest wins

```bash
forgeplan fgr --json | jq '.results[] | select(.formality == 1 and .granularity == 1 and .reliability < 0.6)'
```

Candidates for reliability review; inspect evidence, links and freshness rather than assuming one added pack will complete the artifact.

## Output interpretation

| Field | Meaning |
|---|---|
| `formality` | Validation-check pass fraction |
| `granularity` | Fraction of the five completeness checks satisfied |
| `reliability` | Weighted R_eff, links and freshness, clamped to 0–1 |
| `overall` | Geometric mean of the three axes |
| `grade` | A/B/C/D/F from the boundaries above |

FPF rule thresholds are separate policy settings; there is no `forgeplan fpf trust` command.

## How it fits the workflow

```
health → fgr → spot low-R artifacts → add evidence → score → fgr (re-check)
```

F and G are paid for during Shape (`new → validate`). R is paid for during Evidence
(`new evidence → link → score`). `fgr` is the lens that tells you which bucket needs work.

## See also

- [`forgeplan validate`](/docs/cli/validate/) - rule-level detail for the F axis
- [`forgeplan score`](/docs/cli/score/) - R_eff that feeds the R axis
- [`forgeplan health`](/docs/cli/health/) - separate project-health diagnostics
- [Quality Gates](/docs/methodology/evidence/) - FGR and Trust Calculus
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fgr --help`.

```text
Show F-G-R quality scores (Formality, Granularity, Reliability)

Usage: forgeplan fgr [OPTIONS] [ID]

Arguments:
  [ID]  Artifact ID (scores all if omitted)

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
