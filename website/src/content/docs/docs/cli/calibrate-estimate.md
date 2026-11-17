---
title: forgeplan calibrate-estimate
description: "Compare estimated vs actual hours and measure estimation drift after a sprint."
---

`forgeplan calibrate-estimate` reruns the estimate pipeline for an existing artifact and compares one estimate with positive, finite actual hours. It prints the estimate, confidence, actual hours, ratio (`actual / estimated`), accuracy, and a recommendation. It does not save a calibration dataset or update configuration.

## When to use

- After finishing a PRD/RFC, compare actual hours with its current estimate.
- After a large overrun or underrun, inspect the ratio while context is fresh.
- Save output yourself if you want to aggregate comparisons or tune `estimate.grade_profile`.

## When NOT to use

- Actual hours are incomplete; any comparison is provisional.
- The artifact has no extractable FR/Phase items; the command rejects it.

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

The output includes `Estimated: …h (confidence …%)`, `Actual: …h`, `Ratio: …x (actual / estimated)`, and `Accuracy: …%`. Values depend on the artifact and estimate configuration.

### Compare against a grade

```bash
forgeplan calibrate-estimate PRD-001 --actual-hours 18 --grade senior
```

Without `--grade`, the comparison uses `total_score`, not the senior-hours total. The implementation matches a case-insensitive substring of a grade's debug name; an unmatched value silently falls back to `total_score`. Use a full grade name such as `senior` or `middle` to avoid that fallback.

### Aggregate across the sprint

```bash
for id in PRD-001 PRD-002 PRD-003; do
  forgeplan calibrate-estimate "$id" --actual-hours "$(cat actuals/$id.txt)"
done
```

Pipe results to a script to compute average drift per grade/domain and propose new
multipliers for `.forgeplan/config.yaml`.

## Output interpretation

The ratio is actual hours divided by estimated hours. Accuracy is `100 × min(ratio, 1 / ratio)`.

Recommendations use these branches, in order: ratio `> 1.5` suggests a buffer; `> 1.1` reports slight optimism; `< 0.5` reports very conservative estimates; `< 0.7` reports faster completion; otherwise it reports good calibration. These are CLI messages, not automatic changes to depth or grade multipliers. Compare several completed artifacts before changing your configuration.

## How it fits the workflow

```
sprint close → calibrate-estimate (per artifact) → tune grade_profile → next sprint sharper
```

Think of this as telemetry for your own estimation. The CLI doesn't auto-adjust config -
you review the drift and decide what to change.

## See also

- [`forgeplan estimate`](/docs/cli/estimate/) - the estimate this calibrates against
- [Configuration](/docs/getting-started/configuration/) - where `grade_profile` lives
- [Unified Workflow](/docs/guides/git-workflow/) - sprint close rituals
- [CLI overview](/docs/cli/)
