---
title: forgeplan fpf rules
description: "List active FPF rules grouped by action: EXPLORE, INVESTIGATE, EXPLOIT"
---

`forgeplan fpf rules` lists every active rule in the **First Principles Framework** rule engine, grouped by its recommended action bucket: **EXPLORE** (generate more hypotheses), **INVESTIGATE** (gather more evidence for existing hypotheses), or **EXPLOIT** (commit to a proven path).

These are ForgePlan's executable methodology rules, not a complete implementation of upstream FPF. [`fpf check`](/docs/cli/fpf-check/) evaluates them per artifact and [`fpf dashboard`](/docs/cli/fpf-dashboard/) provides a project view. A nonempty `fpf.rules` configuration replaces the built-in set.

## When to use

- **Before a sprint** - skim the rule set to understand what the engine is looking for.
- **When a rule fires unexpectedly** - read the full rule text to understand the trigger.
- **While authoring methodology docs** - reference rules by ID.
- **While debugging `fpf check`** output - see the full rule definition behind a match.

## When NOT to use

- For per-artifact matches - use [`fpf check <id>`](/docs/cli/fpf-check/).
- For project-wide action recommendations - use [`fpf dashboard`](/docs/cli/fpf-dashboard/).

## Usage

```text
forgeplan fpf rules [OPTIONS]
```

## Options

```text
      --flat     Flat priority-linear table instead of action-grouped tree
      --json     Output full rule dump as JSON
  -h, --help     Print help
  -V, --version  Print version
```

The default view groups rules by action (EXPLORE / INVESTIGATE / EXPLOIT). `--flat` switches to a single priority-ordered table when you want to scan top-priority rules regardless of action. `--json` emits the full rule dump for MCP clients and audit scripts.

## Examples

```bash
# Default: action-grouped tree
forgeplan fpf rules

# Flat priority-ordered table
forgeplan fpf rules --flat

# Machine-readable full dump
forgeplan fpf rules --json

# Typical review flow
forgeplan fpf rules
forgeplan fpf check PRD-019
forgeplan fpf dashboard
```

## The three action buckets

Rules map onto the explore→investigate→exploit axis from FPF Part B:

- **EXPLORE** - built-in rules flag a draft with R_eff < 0.01 or an active artifact without links.
- **INVESTIGATE** - built-in rules flag weak evidence or medium reliability.
- **EXPLOIT** - the built-in recommendation requires R_eff ≥ 0.7 and overall quality ≥ 0.6; it does not itself authorize activation or supersession.

Each rule has a name, priority, condition, action, and optional message. The built-in set does not count hypotheses or fetch live KB citations.

## How it fits

`rules` reads workspace configuration and lists active rules without evaluating an artifact or loading KB section bodies. Rule evaluation and lifecycle checks are separate operations:

- `fpf check <id>` - per-artifact matches
- `fpf dashboard` - project recommendations
- `forgeplan validate` / `activate` - separate validation and lifecycle checks

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf check`](/docs/cli/fpf-check/) - apply rules to one artifact
- [`forgeplan fpf dashboard`](/docs/cli/fpf-dashboard/) - rules aggregated per context
- [`forgeplan validate`](/docs/cli/validate/) - validation gate that consults integrity rules
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf rules --help`.

```text
List active FPF rules grouped by action (EXPLORE/INVESTIGATE/EXPLOIT)

Usage: forgeplan fpf rules [OPTIONS]

Options:
      --flat     Flat priority-linear table instead of action-grouped tree
      --json     Output full rule dump as JSON
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
