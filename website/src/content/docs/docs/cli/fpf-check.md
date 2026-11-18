---
title: forgeplan fpf check
description: "Check which FPF rules match a given artifact and what action they recommend"
---

`forgeplan fpf check <ID>` evaluates active rules against one artifact. It prints the highest-priority matching rule, other matches in priority order, and optionally unmatched names. Actions are EXPLORE / INVESTIGATE / EXPLOIT; this report does not fetch an FPF section citation for each rule.

It's the per-artifact projection of [`fpf rules`](/docs/cli/fpf-rules/) and [`fpf dashboard`](/docs/cli/fpf-dashboard/).

## When to use

- **Before activating a PRD/RFC/ADR** - confirm no INVESTIGATE rules are still firing.
- **After structural validation** - inspect the active rules' metadata and evidence-quality signals; the built-in rules do not count ADI hypotheses.
- **While debugging a stuck artifact** - inspect matching recommendations without treating every match as a blocking gate.
- **During adversarial review** - use rule output as a structured critique checklist.

## When NOT to use

- For syntactic / structural checks - use [`forgeplan validate`](/docs/cli/validate/).
- For project-wide views - use [`fpf dashboard`](/docs/cli/fpf-dashboard/).
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
# Default: winning rule and other priority-ordered matches
forgeplan fpf check PRD-019

# Show unmatched rules too (useful while debugging why a rule didn't fire)
forgeplan fpf check PRD-019 --verbose

# Machine-readable output for MCP clients and audit scripts
forgeplan fpf check ADR-005 --json
```

## How explore / investigate / exploit thresholds work

`explore_reff`, `investigate_reff`, and `exploit_reff` are configuration thresholds, not three derived reliability scores. The built-in rules use these concrete conditions:

| Rule | Condition | Action |
|---|---|---|
| `blind-spot` | draft and R_eff < 0.01 | EXPLORE |
| `weak-evidence` | active/stale and R_eff < 0.5 | INVESTIGATE |
| `orphan-active` | active and no links | EXPLORE |
| `medium-quality` | 0.5 ≤ R_eff < 0.7 | INVESTIGATE |
| `ready-to-build` | R_eff ≥ 0.7 and overall quality ≥ 0.6 | EXPLOIT |

A nonempty `fpf.rules` configuration replaces the built-in rules. `fpf check` itself neither activates an artifact nor changes lifecycle gates. Follow the applicable validation, ADI, review, and evidence requirements separately.

## How it fits

The selected CLI implementation reads artifact records and relations, resolves active rules, and evaluates enriched metadata. It does not load KB section bodies to justify matches. Do not recreate retired tables to run it. `--json` emits the matched/unmatched/winning result with a summary and next-action hint.

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf rules`](/docs/cli/fpf-rules/) - full rule set
- [`forgeplan fpf dashboard`](/docs/cli/fpf-dashboard/) - project-wide view
- [`forgeplan validate`](/docs/cli/validate/) - structural validation
- [`forgeplan reason`](/docs/cli/reason/) - generate more hypotheses when EXPLORE rules fire
- [`forgeplan score`](/docs/cli/score/) - R_eff inputs behind the thresholds

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf check --help`.

```text
Check which FPF rules match a given artifact

Usage: forgeplan fpf check [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID (e.g. PRD-041)

Options:
      --verbose  Show unmatched rule names too
      --json     Output full RuleCheckResult as JSON
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
