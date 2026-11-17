---
title: forgeplan journal
description: "Show decision artifacts by creation date with current linked-evidence counts and flat R_eff. This is not a lifecycle event log."
---

Show decision artifacts ordered by `created_at`, newest first, with current linked-evidence counts and a flat evidence-helper R_eff calculation. Without a type filter, the journal includes ADR, Note, PRD, RFC, Epic, Spec, Problem, and Solution. It is not a stream of activation, renewal, supersession, or deprecation events; use [`log`](/docs/cli/log/) for mutation history.

## When to use

- Onboarding - "how did we get here?"
- Retrospectives - which decisions aged well vs. got superseded
- `--risk` mode: inspect artifacts with zero linked evidence, R_eff below 0.3, or expired eligible evidence.
- Review the most recently created decisions before preparing a retrospective.

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

Risk mode — not limited to active artifacts:

```bash
forgeplan journal --risk
```

## Output interpretation

Each entry prints its creation date, ID, kind, and title. If evidence is linked, a second line shows `R_eff: … | … evidence` and any risk marker. The output does not include an event column or a superseding-artifact arrow.

`--risk` retains an entry when evidence count is zero, the flat helper R_eff is `< 0.3`, or eligible evidence is stale. It does not filter lifecycle status. Terminal evidence is counted as a linked pack but excluded from the helper's scoring minimum and stale-evidence flag. A zero score therefore does not prove that no evidence is linked.

This flat calculation is not the recursive dependency-aware result of `forgeplan score`. The command prints text only; there is no `--json` option.

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
