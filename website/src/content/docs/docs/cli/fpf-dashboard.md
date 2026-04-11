---
title: forgeplan fpf dashboard
description: "Show FPF dashboard — bounded contexts, quality scores, and explore/investigate/exploit recommendations"
---

`forgeplan fpf dashboard` renders a single-screen overview of how the project looks through the **First Principles Framework** lens: which bounded contexts are healthy, which carry reasoning debt, and what the rule engine recommends next (explore more options, investigate existing evidence, or exploit a proven path).

It's the FPF counterpart to `forgeplan health` — `health` reports artifact-level facts (orphans, stale, blind spots); `fpf dashboard` interprets them through trust calculus and the explore/exploit model.

## When to use

- **On session start**, alongside `forgeplan health` — understand where reasoning effort should go today.
- **At sprint planning** — pick the next bounded context to advance based on R_eff and explore/exploit signals.
- **Before a major decision** — see whether you're prematurely exploiting a low-trust path.
- **After a batch of activations** — confirm scores moved in the expected direction.

## When NOT to use

- For raw artifact stats (counts, orphans) — use `forgeplan health`.
- For a single artifact's rules — use [`forgeplan fpf check <id>`](/docs/cli/fpf-check/).
- For KB ingest state — use [`forgeplan fpf status`](/docs/cli/fpf-status/).

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

The dashboard groups output into three blocks:

1. **Bounded contexts** — each context (e.g. `search`, `scoring`, `fpf-kb`) with aggregate R_eff, artifact count, and blind-spot flags.
2. **Quality scores** — explore_reff / investigate_reff / exploit_reff per context, showing where trust is strongest.
3. **Recommended actions** — one of `EXPLORE` (not enough hypotheses), `INVESTIGATE` (hypotheses exist but evidence is weak), or `EXPLOIT` (evidence is strong, ship it).

The action recommendation comes from the same rule engine surfaced by [`forgeplan fpf rules`](/docs/cli/fpf-rules/).

## How it fits

`fpf dashboard` is the visual synthesis of PRD-041 (FPF rule engine) and PRD-043 (methodology integrity). It reads:

- Artifact metadata (kind, status, R_eff) from LanceDB
- Active rules from the FPF KB
- Evidence links and congruence levels

...and projects them onto the explore→investigate→exploit axis defined in FPF section B (trust calculus).

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) — parent command
- [`forgeplan fpf rules`](/docs/cli/fpf-rules/) — the rules that feed the dashboard
- [`forgeplan fpf check`](/docs/cli/fpf-check/) — per-artifact rule match
- [`forgeplan health`](/docs/cli/health/) — artifact-level counterpart
- [Methodology guide](/docs/methodology/overview/)
