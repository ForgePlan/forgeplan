---
title: forgeplan calibrate-estimate
description: "Compare estimated vs actual hours and measure estimation drift after a sprint."
---

`forgeplan calibrate-estimate` reruns the estimate and compares one estimated total
with positive, finite actual hours. `--grade` selects a grade total; without it,
the command uses the total score. It prints actual/estimated ratio and accuracy,
but does not allocate actual hours per FR, save a calibration dataset or update config.

Without calibration, estimates drift silently: you think you're "senior on backend" but
consistently overshoot 50%. This command surfaces that gap so you can adjust the
multiplier, not just your intuition.

## When to use

- End of sprint: run on every PRD/RFC you closed to build a calibration dataset.
- After a surprise (big over/under-run) - immediate feedback while context is fresh.
- Quarterly review — save the text reports and review profile changes manually; this command has no `--json`.
- Benchmarking a new grade profile - compare actual vs different grade assumptions.

## When NOT to use

- Mid-sprint on an unfinished artifact - actuals must be final.
- On artifacts without FR/Phase items - the original estimate was a guess, not a model.

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

Hypothetical excerpt if the selected estimate is 13.5h (not a captured run):

```text
Estimated:  13.5h
Actual:     18.0h
Ratio:      1.33x (actual / estimated)
Accuracy:   75%
```

### Compare against a different grade

```bash
forgeplan calibrate-estimate PRD-001 --actual-hours 18 --grade middle
```

Hypothetical excerpt if the middle-grade estimate is 22h:

```text
Estimated:  22.0h
Actual:     18.0h
Ratio:      0.82x (actual / estimated)
Accuracy:   82%
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

`ratio = actual / estimated`; accuracy is `min(ratio, 1 / ratio) × 100%`.
The implementation gives optimistic-estimate advice above 1.1x and stronger
buffer advice above 1.5x; below 0.7x it warns about conservative estimates,
with stronger advice below 0.5x.

These are estimation heuristics, not proof of a depth mistake or scope change.
Inspect what actually changed before tuning a profile. A larger, comparable
set of completed artifacts is more useful than a single result.

## How it fits the workflow

```
sprint close → calibrate-estimate (per artifact) → tune grade_profile → next sprint sharper
```

Think of this as telemetry for your own estimation. The CLI doesn't auto-adjust config -
you review the drift and decide what to change.

## See also

- [`forgeplan estimate`](/docs/cli/estimate/) - the estimate this calibrates against
- [`forgeplan config`](/docs/getting-started/configuration/) - where grade_profile lives
- [Unified Workflow](/docs/guides/git-workflow/) - sprint close rituals
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan calibrate-estimate --help`.

```text
Compare estimated vs actual hours — calibrate estimation accuracy

Usage: forgeplan calibrate-estimate [OPTIONS] --actual-hours <ACTUAL_HOURS> <ID>

Arguments:
  <ID>  Artifact ID to calibrate

Options:
      --actual-hours <ACTUAL_HOURS>  Actual hours spent
      --grade <GRADE>                Grade to compare (junior, mid, senior). Defaults to total score
  -h, --help                         Print help
  -V, --version                      Print version
```
<!-- forgeplan-runtime:end -->
