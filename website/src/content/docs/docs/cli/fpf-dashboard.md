---
title: forgeplan fpf dashboard
description: "Show FPF dashboard - bounded contexts, quality scores, and explore/investigate/exploit recommendations"
---

`forgeplan fpf dashboard` combines graph-derived bounded contexts, F-G-R grades, rule-engine recommendations, and lifecycle counts. It is a text report, not an automatic reasoning or implementation agent. Use `health` for orphans, expired artifacts, and blind spots.

## When to use

- **On session start**, alongside `forgeplan health` - understand where reasoning effort should go today.
- **At sprint planning** - inspect artifact clusters and pending rule recommendations.
- **Before a major decision** - see whether you're prematurely exploiting a low-trust path.
- **After a batch of activations** - confirm scores moved in the expected direction.

## When NOT to use

- For raw artifact stats (counts, orphans) - use `forgeplan health`.
- For a single artifact's rules - use [`forgeplan fpf check <id>`](/docs/cli/fpf-check/).
- For KB ingest state - use [`forgeplan fpf status`](/docs/cli/fpf-status/).

## Usage

```text
forgeplan fpf dashboard
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

The dashboard has four sections:

1. **Bounded Contexts** — graph clusters with name, member count, cohesion percentage, and member IDs. These are inferred from relations, not a configured domain taxonomy.
2. **Quality (F-G-R)** — artifact counts by grade, not per-context explore/investigate/exploit scores.
3. **Next Actions** — counts by `EXPLORE`, `INVESTIGATE`, and `EXPLOIT`, followed by up to ten artifact recommendations and reasons.
4. **Pipeline** — total artifacts, active count, and draft count.

It reads records and relations from LanceDB and workspace FPF configuration. A nonempty `fpf.rules` list replaces built-in rules; no ingested FPF knowledge base or LLM provider is needed. Recommendations do not grant activation or supersession permission; those operations retain their own checks. Dashboard freshness parsing accepts the full `%Y-%m-%dT%H:%M:%S` form; date-only validity values are not treated as stale here, unlike the standalone `fgr` command.

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf rules`](/docs/cli/fpf-rules/) - the rules that feed the dashboard
- [`forgeplan fpf check`](/docs/cli/fpf-check/) - per-artifact rule match
- [`forgeplan health`](/docs/cli/health/) - artifact-level counterpart
- [Methodology guide](/docs/methodology/overview/)
