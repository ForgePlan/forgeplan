---
title: forgeplan fpf dashboard
description: "Show FPF dashboard - bounded contexts, quality scores, and explore/investigate/exploit recommendations"
---

`forgeplan fpf dashboard` renders a single-screen overview of how the project looks through the **First Principles Framework** lens: which bounded contexts are healthy, which carry reasoning debt, and what the rule engine recommends next (explore more options, investigate existing evidence, or exploit a proven path).

It's the FPF counterpart to `forgeplan health` - `health` reports artifact-level facts (orphans, stale, blind spots); `fpf dashboard` interprets them through trust calculus and the explore/exploit model.

## When to use

- **On session start**, alongside `forgeplan health` - understand where reasoning effort should go today.
- **At sprint planning** - pick the next bounded context to advance based on R_eff and explore/exploit signals.
- **Before a major decision** - see whether you're prematurely exploiting a low-trust path.
- **After a batch of activations** - confirm scores moved in the expected direction.

## When NOT to use

- For raw artifact stats (counts, orphans) - use `forgeplan health`.
- For a single artifact's rules - use [`forgeplan fpf check <id>`](/docs/cli/fpf-check/).
- For KB ingest state - use [`forgeplan fpf status`](/docs/cli/fpf-status/).

## Usage

```text
forgeplan fpf dashboard [OPTIONS]
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
# Full dashboard
forgeplan fpf dashboard

# Typical session-start combo
forgeplan health
forgeplan fpf dashboard
forgeplan blocked
```

## What you see

The dashboard displays four blocks:

1. **Bounded contexts** - graph clusters, members and cohesion.
2. **Quality (F-G-R)** - counts by ForgePlan's Formality/Granularity/Reliability grade.
3. **Next Actions** - per-artifact `EXPLORE`, `INVESTIGATE` or `EXPLOIT` recommendations.
4. **Pipeline** - a pipeline status summary.

The action recommendation comes from the same rule engine surfaced by [`forgeplan fpf rules`](/docs/cli/fpf-rules/).

## How it fits

`fpf dashboard` is the visual synthesis of PRD-041 (FPF rule engine) and PRD-043 (methodology integrity). It reads:

- Artifact metadata (kind, status, R_eff) from the selected workspace
- Rules from workspace configuration or built-in defaults, not the ingested spec
- Evidence links and congruence levels

These are product heuristics, not upstream FPF's literal scales. Help registration alone does not establish runtime acceptance for every workspace layout.

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf rules`](/docs/cli/fpf-rules/) - the rules that feed the dashboard
- [`forgeplan fpf check`](/docs/cli/fpf-check/) - per-artifact rule match
- [`forgeplan health`](/docs/cli/health/) - artifact-level counterpart
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf dashboard --help`.

```text
Show FPF dashboard — bounded contexts, quality scores, explore-exploit actions

Usage: forgeplan fpf dashboard

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
