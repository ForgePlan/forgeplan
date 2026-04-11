---
title: forgeplan journal
description: "Decision journal — chronological timeline of activations with R_eff"
---

Show the **decision journal** — a chronological timeline of when artifacts
were activated, superseded, deprecated, or renewed, annotated with their
R_eff scores. Unlike [`log`](/docs/cli/log/), which records every mutation,
`journal` shows only decision events — the story you tell a new teammate.

## When to use

- Onboarding — "how did we get here?"
- Retrospectives — which decisions aged well vs. got superseded
- `--risk` mode: find active decisions without evidence (blind spots) on a
  time axis
- Preparing release notes — extract everything activated since tag

## Not to use when

- You want low-level CRUD events → use [`forgeplan log`](/docs/cli/log/)
- You want current snapshot → use [`forgeplan health`](/docs/cli/health/)

## Usage

```text
forgeplan journal [OPTIONS]
```

## Options

```text
  -t, --type <TYPE>  Filter by kind (adr, note, problem, solution)
      --risk         Show only at-risk decisions (no evidence, stale, low R_eff)
  -h, --help         Print help
  -V, --version      Print version
```

## Examples

Full decision timeline:

```bash
forgeplan journal
```

Only ADRs — architectural record, smaller set:

```bash
forgeplan journal --type adr
```

Risk mode — active decisions without evidence or with stale proof:

```bash
forgeplan journal --risk
```

## Output interpretation

One line per decision event, newest first:

```
2026-04-11  ACTIVATE    PRD-046  R_eff=0.85   Docs v0.18.0 catch-up
2026-04-09  ACTIVATE    ADR-007  R_eff=1.00   Choose bm25 crate v2.3.2
2026-04-08  SUPERSEDE   PRD-018  → PRD-042    FPF KB vector search
2026-04-07  DEPRECATE   RFC-003  R_eff=0.10   (replaced by RFC-004)
```

| Column    | Meaning                                                    |
|-----------|------------------------------------------------------------|
| Date      | Date of the lifecycle event                                |
| Event     | ACTIVATE / SUPERSEDE / DEPRECATE / RENEW / REOPEN          |
| Artifact  | ID of the decision                                         |
| R_eff     | Effective trust (`min(evidence_scores)`), 0.00–1.00        |
| Title     | Short title                                                |

`R_eff = 0.00` on an active decision is a red flag — it means no evidence is
linked at all. `--risk` surfaces exactly these.

## How it fits

`journal` sits between `log` (raw mutations) and `health` (current rollup):

```
log         → journal    → health
everything    decisions    current state
```

Use it to tell the decision story; use `health` to see where you are now.

## See also

- [`forgeplan log`](/docs/cli/log/) — full mutation trail
- [`forgeplan health`](/docs/cli/health/) — current state dashboard
- [`forgeplan score`](/docs/cli/score/) — R_eff for one artifact
- [`forgeplan blindspots`](/docs/cli/blindspots/) — all at-risk decisions at once
