---
title: forgeplan fpf check
description: "Check which FPF rules match a given artifact and what action they recommend"
---

`forgeplan fpf check <ID>` evaluates every active **First Principles Framework** rule against a single artifact and prints the matches — grouped by action (EXPLORE / INVESTIGATE / EXPLOIT) and annotated with the FPF section each rule is derived from.

It's the per-artifact projection of [`fpf rules`](/docs/cli/fpf-rules/) and [`fpf dashboard`](/docs/cli/fpf-dashboard/).

## When to use

- **Before activating a PRD/RFC/ADR** — confirm no INVESTIGATE rules are still firing.
- **When `forgeplan validate` passes but you want deeper reasoning review** — rules catch things validation can't (e.g. "too few hypotheses for Deep depth").
- **While debugging a stuck artifact** — see exactly which rule is blocking progress.
- **During adversarial review** — use rule output as a structured critique checklist.

## When NOT to use

- For syntactic / structural checks — use [`forgeplan validate`](/docs/cli/validate/).
- For project-wide views — use [`fpf dashboard`](/docs/cli/fpf-dashboard/).
- On Tactical-depth items where FPF reasoning is deliberately bypassed.

## Usage

```text
forgeplan fpf check [OPTIONS] <ID>
```

## Arguments

```text
  <ID>   Artifact ID (e.g. PRD-041)
```

## Options

```text
      --verbose  Show unmatched rule names too
      --json     Output full RuleCheckResult as JSON
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
# Default: show matched rules grouped by action
forgeplan fpf check PRD-019

# Show unmatched rules too (useful while debugging why a rule didn't fire)
forgeplan fpf check PRD-019 --verbose

# Machine-readable output for MCP clients and audit scripts
forgeplan fpf check ADR-005 --json
```

## How explore / investigate / exploit thresholds work

Each bounded context the artifact belongs to has three derived R_eff values:

- **`explore_reff`** — how well-covered the hypothesis space is. Low → EXPLORE rules fire ("generate alternatives").
- **`investigate_reff`** — how strong the evidence is for the hypotheses that exist. Low → INVESTIGATE rules fire ("attach better evidence").
- **`exploit_reff`** — whether the chosen path clears the "ready to commit" bar. High → EXPLOIT rules unlock activation.

Thresholds are calibrated per depth:

| Depth     | Needs EXPLORE clear | Needs INVESTIGATE clear | Needs EXPLOIT clear |
|-----------|---------------------|-------------------------|---------------------|
| Tactical  | —                   | —                       | —                   |
| Standard  | recommended         | recommended             | required            |
| Deep      | required            | required                | required            |
| Critical  | required + review   | required + review       | required + review   |

For Deep/Critical artifacts, `fpf check` is effectively a pre-activation gate: fix INVESTIGATE rules before calling `forgeplan activate`.

## How it fits

`fpf check` reads:

1. The artifact row from LanceDB (kind, depth, status, R_eff, linked evidence).
2. The active rule set from the FPF rule engine (PRD-041).
3. FPF KB sections cited by each rule (PRD-042).

...and produces a human-readable report plus a machine-readable summary consumed by `fpf dashboard` and `forgeplan validate` (PRD-043 methodology integrity).

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) — parent command
- [`forgeplan fpf rules`](/docs/cli/fpf-rules/) — full rule set
- [`forgeplan fpf dashboard`](/docs/cli/fpf-dashboard/) — project-wide view
- [`forgeplan validate`](/docs/cli/validate/) — structural validation
- [`forgeplan reason`](/docs/cli/reason/) — generate more hypotheses when EXPLORE rules fire
- [`forgeplan score`](/docs/cli/score/) — R_eff inputs behind the thresholds
