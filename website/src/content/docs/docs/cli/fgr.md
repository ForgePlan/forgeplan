---
title: forgeplan fgr
description: "Show F-G-R quality scores — Formality, Granularity, Reliability — orthogonal to lifecycle."
---

`forgeplan fgr` prints the **F-G-R** quality triple for an artifact:

- **F — Formality**: how structured is it? Filled MUST sections, linked parents, explicit acceptance criteria.
- **G — Granularity**: how decomposed is it? FR count, phase checkboxes, leaf-level detail.
- **R — Reliability**: how trusted is it? Linked evidence, R_eff, adversarial review status.

Each axis is scored 0–3. F-G-R is **orthogonal to lifecycle** — a `draft` artifact can
already be F=3/G=3/R=2, and an `active` one can be F=1/G=1/R=0 (a blind spot). The FPF
Trust Calculus uses `exploit_fgr ≥ 0.6` as the threshold to reuse a decision without
re-deriving it.

## When to use

- Quick visual check of maturity without running the full 30+ validator rules.
- Session start, after `health`, to sort artifacts by "cheapest to level up".
- Comparing two candidate PRDs before committing scope.
- Feeding the Trust Calculus: `exploit_fgr` gating explore-vs-exploit decisions in FPF rule engine.

## When NOT to use

- As a substitute for `validate` — FGR is a summary, not a rule report.
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

Output:

```text
PRD-001 — Auth System
  F = 3/3   Formality     (all MUST sections filled, parents linked)
  G = 2/3   Granularity   (7 FR, 2 phases — phases missing acceptance criteria)
  R = 1/3   Reliability   (1 evidence linked, R_eff = 0.40)
  overall = 2.0/3   exploit_fgr = 0.67
```

`exploit_fgr` ≥ 0.6 → FPF rule engine will trust this decision for reuse.

### Rank all artifacts by maturity

```bash
forgeplan fgr --json | jq 'sort_by(.overall) | reverse | .[0:10]'
```

Top 10 most mature artifacts — useful when deciding what to promote or cite.

### Find the cheapest wins

```bash
forgeplan fgr --json | jq '.[] | select(.F == 3 and .G == 3 and .R < 2)'
```

Artifacts that only need evidence to reach full maturity — add evidence, score, done.

## Output interpretation

| Axis | 0 | 1 | 2 | 3 |
|------|---|---|---|---|
| **F** — Formality    | stub, MUST missing | some MUSTs filled | all MUSTs + aliases | all MUSTs + links + ACs |
| **G** — Granularity  | no FR/phases | 1–3 FR | 4–7 FR + phases | 8+ FR + phased + measurable |
| **R** — Reliability  | no evidence, R_eff=0 | 1 evidence, CL ≤ 1 | 2+ evidence, R_eff ≥ 0.4 | strong evidence, R_eff ≥ 0.8 |

**Overall** = average of F/G/R. **exploit_fgr** = overall / 3.0, used as gate in
`forgeplan fpf trust`.

## How it fits the workflow

```
health → fgr → spot low-R artifacts → add evidence → score → fgr (re-check)
```

F and G are paid for during Shape (`new → validate`). R is paid for during Evidence
(`new evidence → link → score`). `fgr` is the lens that tells you which bucket needs work.

## See also

- [`forgeplan validate`](/docs/cli/validate/) — rule-level detail for the F axis
- [`forgeplan score`](/docs/cli/score/) — R_eff that feeds the R axis
- [`forgeplan health`](/docs/cli/health/) — project-wide F-G-R aggregate
- [Quality Gates](/docs/methodology/evidence/) — FGR and Trust Calculus
- [CLI overview](/docs/cli/)
