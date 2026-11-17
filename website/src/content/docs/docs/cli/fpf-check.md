---
title: forgeplan fpf check
description: "Check which FPF rules match a given artifact and what action they recommend"
---

`forgeplan fpf check <ID>` evaluates the active **First Principles Framework** rules for one artifact. It prints the winning rule first, then other matches in ascending priority order; it does not group by action or load cited KB sections.

It's the per-artifact projection of [`fpf rules`](/docs/cli/fpf-rules/) and [`fpf dashboard`](/docs/cli/fpf-dashboard/).

## When to use

- **Before activating a PRD/RFC/ADR** - inspect the reasoning recommendation alongside the separate lifecycle review.
- **When structural validation is not enough** - inspect configured rule conditions and their matched/unmatched results.
- **While debugging a recommendation** - see the first matching rule that wins.
- **During adversarial review** - use rule output as a structured critique checklist.

## When NOT to use

- For syntactic / structural checks - use [`forgeplan validate`](/docs/cli/validate/).
- For project-wide views - use [`fpf dashboard`](/docs/cli/fpf-dashboard/).
- To grant activation permission: recommendations are not lifecycle guards, at any depth.

## Usage

```text
forgeplan fpf check [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID (e.g. PRD-041)
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

## Rules, output, and limits

Non-empty `fpf.rules` in `.forgeplan/config.yaml` replaces the default rule
set. Rules are sorted by ascending numeric priority, and the first match wins.
Conditions use stored artifact data and derived trust/link/F-G-R signals;
`explore_reff`, `investigate_reff`, and `exploit_reff` are configuration
thresholds, not three separate per-context scores. There is no universal
depth-by-action gate table enforced by this command.

`--json` contains `artifact_id`, `kind`, `status`, `matched`, `unmatched`,
`winning`, `summary`, and `_next_action`. Each matched/winning rule has
`name`, `priority`, `action`, and `message`; `winning` is null when none
matches. `unmatched` is an array of rule names and is present in JSON
regardless of `--verbose`.

The suggested command is `activate` for EXPLOIT, `reason` for EXPLORE or
INVESTIGATE, or `get` when no rule matches. These are advice, not execution.
Checking succeeds with exit `0` regardless of the recommended action;
missing artifacts or checking failures exit `1`. The command requires a
workspace, but not an ingested FPF knowledge base. Run `review` separately
for activation findings; `activate` still applies its own guards.


## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf rules`](/docs/cli/fpf-rules/) - full rule set
- [`forgeplan fpf dashboard`](/docs/cli/fpf-dashboard/) - project-wide view
- [`forgeplan validate`](/docs/cli/validate/) - structural validation
- [`forgeplan reason`](/docs/cli/reason/) - generate more hypotheses when EXPLORE rules fire
- [`forgeplan score`](/docs/cli/score/) - R_eff inputs behind the thresholds
