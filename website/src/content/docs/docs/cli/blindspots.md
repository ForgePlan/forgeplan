---
title: forgeplan blindspots
description: "Decisions without linked evidence and isolated artifacts - health triage"
---

Find **blind spots** — eligible decision artifacts without linked evidence — and **orphans**, which have neither incoming nor outgoing relations. Missing evidence and `R_eff = 0` are different checks: a linked pack can exist but fail to support the decision. Draft, deprecated and superseded decisions are excluded from blind-spot detection.

## When to use

- **Session start** - run this right after `forgeplan health` and fix before
  starting new work (do not accumulate debt)
- **CI reporting** — inspect findings; use `health --strict` for a separate exit-code gate
- **Pre-release** - zero blind spots before cutting a tag
- **Retro** - count how many blind spots accumulated over the sprint

## Not to use when

- You want _stuck_ items (different problem) → use [`forgeplan blocked`](/docs/cli/blocked/)
- You want overall rollup → use [`forgeplan health`](/docs/cli/health/)
- You want decision timeline → use [`forgeplan journal`](/docs/cli/journal/) with `--risk`

## Usage

```text
forgeplan blindspots
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

Print all blind spots:

```bash
forgeplan blindspots
```

For an exit-code health gate, use `health --strict`; `blindspots` itself prints findings without failing merely because they exist:

```bash
forgeplan health --strict
```

Combined with [`health --ci`](/docs/cli/health/) for full gating:

```bash
forgeplan health --ci && forgeplan blindspots
```

## Output interpretation

Illustrative findings (not a literal CLI transcript or measurements from your workspace):

```
BLIND DECISIONS (no linked evidence)
  PRD-046  Docs v0.18.0 catch-up    → create EvidencePack + link
  ADR-008  Cloudflare Pages choice  → create EvidencePack + link

ORPHANS (no incoming or outgoing links)
  PRD-044  Unused PRD               → link to Epic or deprecate
  NOTE-031 One-off observation      → link to artifact or ignore

Summary: 2 blind decisions, 2 orphans
```

| Section         | What it means                                         |
|-----------------|-------------------------------------------------------|
| Blind decisions | Eligible decisions with no linked evidence |
| Orphans         | Artifact with neither incoming nor outgoing relations |

`blindspots` returns success after reporting findings. `health --ci` and `health --strict` have their own documented failure rules; do not infer them from this command's output.

## How it fits

`blindspots` extracts two classes from [`health`](/docs/cli/health/) for triage.
Use a health command with explicit failure semantics for CI.
The workflow recommendation is:

```
session start → health → blindspots → (fix) → new work
```

If `blindspots` is not empty, _fix it first_. Never let active decisions
drift without measurement - that is exactly what the R_eff weakest-link
model is designed to prevent.

## See also

- [`forgeplan health`](/docs/cli/health/) - full project dashboard (with
  `--ci` mode)
- [`forgeplan journal --risk`](/docs/cli/journal/) - time-axis view of
  blind decisions
- [`forgeplan score`](/docs/cli/score/) - R_eff for one artifact
- [`forgeplan new evidence`](/docs/cli/new/) - create an EvidencePack
- [`forgeplan link`](/docs/cli/link/) - attach evidence with `--relation informs`

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan blindspots --help`.

```text
Show blind spots — decisions without evidence, orphan artifacts

Usage: forgeplan blindspots

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
