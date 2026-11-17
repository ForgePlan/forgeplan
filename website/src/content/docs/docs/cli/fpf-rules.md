---
title: forgeplan fpf rules
description: "List active FPF rules grouped by action: EXPLORE, INVESTIGATE, EXPLOIT"
---

`forgeplan fpf rules` lists every active rule in the **First Principles Framework** rule engine, grouped by its recommended action bucket: **EXPLORE** (generate more hypotheses), **INVESTIGATE** (gather more evidence for existing hypotheses), or **EXPLOIT** (commit to a proven path).

Rules come from a nonempty workspace `fpf.rules` configuration or built-in defaults, not the ingested FPF knowledge base. They drive [`fpf check`](/docs/cli/fpf-check/) and dashboard recommendations. Evaluation is priority-ascending, first match wins; an action label is advice, not lifecycle permission.

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

- **EXPLORE** — recommend investigating alternatives.
- **INVESTIGATE** — recommend gathering or refreshing support.
- **EXPLOIT** — recommend acting on available support, without bypassing validation or lifecycle guards.

The actual predicates and thresholds are in the active rules. Do not assume universal hypothesis counts or R_eff cutoffs. Each rule has a name, priority, condition, action, and optional message. JSON contains `source` (`Config` or `Default`), `count`, `rules`, and `_next_action`; rule objects contain `name`, `priority`, `action`, `condition`, `condition_summary`, and `message`. `--json` takes precedence over `--flat`.

## How it fits

Use `fpf check <id>` to inspect matched and winning rules and `fpf dashboard` to see project recommendations. Listing rules is read-only and requires workspace configuration, not a provider or KB ingest. Validation and activation have their own checks; an `EXPLOIT` recommendation does not replace them.

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf check`](/docs/cli/fpf-check/) - apply rules to one artifact
- [`forgeplan fpf dashboard`](/docs/cli/fpf-dashboard/) - graph clusters, grades, and artifact recommendations
- [`forgeplan validate`](/docs/cli/validate/) - artifact validation rules
- [Methodology guide](/docs/methodology/overview/)
