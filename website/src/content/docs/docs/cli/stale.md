---
title: forgeplan stale
description: "List artifacts with expired valid_until — the refresh backlog at a glance."
---

`forgeplan stale` finds every artifact whose `valid_until` is in the past. These are
the "stale" candidates in the lifecycle state machine — they have not yet been
deprecated, but their evidence is no longer fresh, so R_eff has been capped at 0.1
(stale, not absent).

Unlike `decay`, which previews upcoming expirations, `stale` is reactive: it lists
the ones you already missed. Run it at session start so you can `renew` or `reopen`
them before starting new work.

## When to use

- Session start, right after `forgeplan health` — clear stale debt before new work.
- Brownfield import — see which imported artifacts are already past their expiry.
- Before relying on an ADR — is this decision still valid?
- CI pipeline (with `--json`) — warn if stale artifacts are referenced in new code.

## When NOT to use

- As a gate on `activate` — use `validate` and `review` instead.
- On Notes (they auto-expire after 90 days and are hidden by default anyway).

## Usage

```text
forgeplan stale [OPTIONS]
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Find all stale artifacts

```bash
forgeplan stale
```

Output:

```text
Stale artifacts (valid_until expired)
─────────────────────────────────────
ADR-002  LanceDB schema v2    expired  12d ago   R_eff: 0.90 → 0.10
PRD-007  Search intelligence  expired  45d ago   R_eff: 0.80 → 0.10
ADR-004  Auth token strategy  expired   3d ago   R_eff: 1.00 → 0.10

3 artifacts need renewal or reopening
```

### Machine-readable

```bash
forgeplan stale --json | jq '.[] | select(.days_overdue > 30)'
```

Filter artifacts overdue by more than 30 days — high-priority refresh candidates.

### Session-start triage

```bash
forgeplan health && forgeplan stale
# for each stale artifact: renew (extend) OR reopen (new draft)
forgeplan renew ADR-002 --reason "still valid, extend 6m" --until 2026-10-01
forgeplan reopen PRD-007 --reason "replace with new approach"
```

## Output interpretation

| Column         | Meaning                                                 |
|----------------|---------------------------------------------------------|
| ID / title     | artifact identity                                       |
| expired Xd ago | days past `valid_until`                                 |
| R_eff drop     | previous cached R_eff vs current (capped at 0.1)        |

For each stale artifact you have three options:

1. **`renew`** — the decision is still correct; extend `valid_until` with a new reason.
2. **`reopen`** — the context has changed; create a new draft and deprecate the old one (lineage preserved).
3. **`deprecate`** — the decision no longer applies; mark it terminal with a reason.

## How it fits the workflow

```
session start → health → stale → renew | reopen | deprecate → start new work
```

Stale is the "don't start on fresh work while debt accumulates" guardrail. The
Unified Workflow protocol treats stale clearance as mandatory before picking up P0 tasks.

## See also

- [`forgeplan decay`](/docs/cli/decay/) — preview upcoming expirations
- [`forgeplan renew`](/docs/cli/renew/) — extend valid_until for still-valid decisions
- [`forgeplan reopen`](/docs/cli/reopen/) — replace stale with a new draft (lineage)
- [`forgeplan deprecate`](/docs/cli/deprecate/) — terminal state for no-longer-valid decisions
- [CLI overview](/docs/cli/)
