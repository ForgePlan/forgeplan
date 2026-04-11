---
title: forgeplan calibrate-estimate
description: "Compare estimated vs actual hours and measure estimation drift after a sprint."
---

`forgeplan calibrate-estimate` closes the loop on `forgeplan estimate`. You pass the
actual hours you spent on an artifact, and it reports how far the estimate was off —
per grade, per FR, and overall. Over time this feeds back into your `grade_profile`
config so estimates get sharper.

Without calibration, estimates drift silently: you think you're "senior on backend" but
consistently overshoot 50%. This command surfaces that gap so you can adjust the
multiplier, not just your intuition.

## When to use

- End of sprint: run on every PRD/RFC you closed to build a calibration dataset.
- After a surprise (big over/under-run) — immediate feedback while context is fresh.
- Quarterly review — aggregate `--json` output to tune `estimate.grade_profile` in config.
- Benchmarking a new grade profile — compare actual vs different grade assumptions.

## When NOT to use

- Mid-sprint on an unfinished artifact — actuals must be final.
- On artifacts without FR/Phase items — the original estimate was a guess, not a model.

## Usage

```text
forgeplan calibrate-estimate [OPTIONS] --actual-hours <ACTUAL_HOURS> <ID>
```

## Arguments

```text
  <ID>  Artifact ID to calibrate
```

## Options

```text
      --actual-hours <ACTUAL_HOURS>  Actual hours spent
      --grade <GRADE>                Grade to compare (junior, mid, senior). Defaults to total score
  -h, --help                         Print help
  -V, --version                      Print version
```

## Examples

### Calibrate one PRD after sprint close

```bash
forgeplan calibrate-estimate PRD-001 --actual-hours 18
```

Output:

```text
PRD-001 — Auth System
  estimated (senior):    13.5h
  actual:                18.0h
  drift:                 +33.3%  (over)
  verdict:               estimate undershot — adjust senior backend multiplier +15%
```

### Compare against a different grade

```bash
forgeplan calibrate-estimate PRD-001 --actual-hours 18 --grade middle
```

Output:

```text
estimated (middle):   22.0h
actual:               18.0h
drift:                -18.2%  (under)
verdict:              you performed between middle and senior on this one
```

### Aggregate across the sprint

```bash
for id in PRD-001 PRD-002 PRD-003; do
  forgeplan calibrate-estimate "$id" --actual-hours "$(cat actuals/$id.txt)"
done
```

Pipe results to a script to compute average drift per grade/domain and propose new
multipliers for `.forgeplan/config.yaml`.

## Output interpretation

| Drift range     | Interpretation                                |
|-----------------|-----------------------------------------------|
| within ±15%     | estimate is accurate — no tuning needed       |
| +15% … +40%     | under-estimated — increase grade multiplier   |
| > +40%          | depth was wrong, not estimate — escalate      |
| -15% … -40%     | over-estimated — decrease multiplier          |
| < -40%          | scope was cut or estimate inflated            |

Track drift per domain (backend, frontend, devops, ai_ml). A single PRD is noise;
ten PRDs is signal.

## How it fits the workflow

```
sprint close → calibrate-estimate (per artifact) → tune grade_profile → next sprint sharper
```

Think of this as telemetry for your own estimation. The CLI doesn't auto-adjust config —
you review the drift and decide what to change.

## See also

- [`forgeplan estimate`](/docs/cli/estimate/) — the estimate this calibrates against
- [`forgeplan config`](/docs/getting-started/configuration/) — where grade_profile lives
- [Unified Workflow](/docs/guides/git-workflow/) — sprint close rituals
- [CLI overview](/docs/cli/)
