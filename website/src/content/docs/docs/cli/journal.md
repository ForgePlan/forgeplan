---
title: forgeplan journal
description: "Decision artifacts in creation order with current evidence scores and risk indicators."
---

Show decision artifacts in reverse creation order with current evidence scores and
risk indicators. This is not a lifecycle-event log: it does not list separate
ACTIVATE, SUPERSEDE or RENEW events. Use [`log`](/docs/cli/log/) for mutation history.

## When to use

- Onboarding - "how did we get here?"
- Retrospectives - which decisions aged well vs. got superseded
- `--risk` mode: find active decisions without evidence (blind spots) on a
  time axis
- Reviewing recently created decisions; use `release-notes` for Git-ref-based reporting

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

Only ADRs - architectural record, smaller set:

```bash
forgeplan journal --type adr
```

Risk mode - active decisions without evidence or with stale proof:

```bash
forgeplan journal --risk
```

## Output interpretation

One entry per selected decision artifact, newest `created_at` first.
Illustrative output, not a runtime transcript:

```text
  2026-04-11  ADR-007 [adr] "Storage engine"
         R_eff: 0.90 | 1 evidence
```

| Column    | Meaning                                                    |
|-----------|------------------------------------------------------------|
| Date | Artifact creation date |
| Artifact | Decision ID and kind |
| R_eff | Current flat linked-evidence calculation, not the recursive score report |
| Title     | Short title                                                |

Zero R_eff warrants review: it can mean no eligible support or refuting evidence,
not necessarily no links. `--risk` includes no-evidence, stale-evidence and low-score entries.

## How it fits

`journal` sits between `log` (raw mutations) and `health` (current rollup):

```
log         → journal    → health
everything    decisions    current state
```

Use it to tell the decision story; use `health` to see where you are now.

## See also

- [`forgeplan log`](/docs/cli/log/) - full mutation trail
- [`forgeplan health`](/docs/cli/health/) - current state dashboard
- [`forgeplan score`](/docs/cli/score/) - R_eff for one artifact
- [`forgeplan blindspots`](/docs/cli/blindspots/) - all at-risk decisions at once

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan journal --help`.

```text
Show decision journal — chronological timeline with R_eff scores

Usage: forgeplan journal [OPTIONS]

Options:
  -t, --type <TYPE>  Filter by kind (adr, note, problem, solution)
      --risk         Show only at-risk decisions (no evidence, stale, low R_eff)
  -h, --help         Print help
  -V, --version      Print version
```
<!-- forgeplan-runtime:end -->
