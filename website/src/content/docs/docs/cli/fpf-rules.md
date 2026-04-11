---
title: forgeplan fpf rules
description: "List active FPF rules grouped by action: EXPLORE, INVESTIGATE, EXPLOIT"
---

`forgeplan fpf rules` lists every active rule in the **First Principles Framework** rule engine, grouped by its recommended action bucket: **EXPLORE** (generate more hypotheses), **INVESTIGATE** (gather more evidence for existing hypotheses), or **EXPLOIT** (commit to a proven path).

The rules encode FPF's trust calculus in machine-executable form. They're what [`fpf check`](/docs/cli/fpf-check/) evaluates against individual artifacts and what [`fpf dashboard`](/docs/cli/fpf-dashboard/) aggregates into project-level recommendations.

## When to use

- **Before a sprint** — skim the rule set to understand what the engine is looking for.
- **When a rule fires unexpectedly** — read the full rule text to understand the trigger.
- **While authoring methodology docs** — reference rules by ID.
- **While debugging `fpf check`** output — see the full rule definition behind a match.

## When NOT to use

- For per-artifact matches — use [`fpf check <id>`](/docs/cli/fpf-check/).
- For project-wide action recommendations — use [`fpf dashboard`](/docs/cli/fpf-dashboard/).

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

- **EXPLORE** — the artifact or context has too few hypotheses. The rule suggests generating alternatives (e.g. "Fewer than 3 hypotheses on Deep depth — run `forgeplan reason`").
- **INVESTIGATE** — hypotheses exist but evidence is weak or low-congruence. The rule suggests gathering more (e.g. "R_eff below threshold — attach a measurement EvidencePack").
- **EXPLOIT** — trust is strong enough to commit. The rule clears the artifact for activation or supersession.

Each rule carries a condition (predicate on artifact metadata), an action label, and a human-readable justification tied to an FPF section (e.g. `ref: B.3`).

## How it fits

The rule engine was introduced in **PRD-041** and extended in **PRD-043** (methodology integrity). It reads artifact state from LanceDB and consults the FPF KB for justification text. Output flows into:

- `fpf check <id>` — per-artifact view
- `fpf dashboard` — aggregated per-context view
- `forgeplan validate` / `activate` gates (for integrity rules)

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) — parent command
- [`forgeplan fpf check`](/docs/cli/fpf-check/) — apply rules to one artifact
- [`forgeplan fpf dashboard`](/docs/cli/fpf-dashboard/) — rules aggregated per context
- [`forgeplan validate`](/docs/cli/validate/) — validation gate that consults integrity rules
- [Methodology guide](/docs/methodology/overview/)
