---
title: forgeplan stale
description: "List artifacts with expired valid_until - the refresh backlog at a glance."
---

`forgeplan stale` lists records with overdue `valid_until`. It does not rewrite
their status or cached R_eff. An expired artifact can still have stored
`status: active`; expiry detection and lifecycle transition are separate.

Evidence scoring applies its own expiry rule when scores are computed. This
listing is not a gradual decay simulation.

## When to use

- Session start, right after `forgeplan health` - clear stale debt before new work.
- Brownfield import - see which imported artifacts are already past their expiry.
- Before relying on an ADR - is this decision still valid?
- CI pipeline (with `--json`) - warn if stale artifacts are referenced in new code.

## When NOT to use

- As a gate on `activate` - use `validate` and `review` instead.
- To apply a fixed Note TTL — this command does not establish automatic 90-day expiry or hide Notes by that rule.

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

The text report prints `ID`, `Title`, `Expired` and `Days ago`.
It does not show a before/after R_eff column. Use `score` to inspect scoring
impact separately; expiry of the decision is not automatically expiry of every supporting pack.

### Machine-readable

```bash
forgeplan stale --json | jq '.stale[] | select(.days_expired > 30)'
```

Filter artifacts overdue by more than 30 days - high-priority refresh candidates.

### Session-start triage

```bash
forgeplan health && forgeplan stale
# Inspect stored status and the reason for expiry before choosing a transition
forgeplan get ADR-002 --json
# Example for stored status stale, after a fresh review justifies the date:
forgeplan renew ADR-002 --reason "reviewed supporting evidence" --until 2027-04-23
forgeplan reopen PRD-007 --reason "replace with new approach"
```

## Output interpretation

| Column         | Meaning                                                 |
|----------------|---------------------------------------------------------|
| ID / title     | artifact identity                                       |
| expired Xd ago | days past `valid_until`                                 |
| Days ago | elapsed days past the parsed date; no R_eff change is performed by listing |

Choose based on both the review and stored lifecycle state:

1. **`renew`** — for a stored `stale` decision after a justified review. An expired record still stored `active` does not permit an `active → active` renewal.
2. **`reopen`** - the context has changed; create a new draft and deprecate the old one (lineage preserved).
3. **`deprecate`** - the decision no longer applies; mark it terminal with a reason.

## How it fits the workflow

```
session start → health → stale → renew | reopen | deprecate → start new work
```

Stale is the "don't start on fresh work while debt accumulates" guardrail. The
Unified Workflow protocol treats stale clearance as mandatory before picking up P0 tasks.

## See also

- [`forgeplan decay`](/docs/cli/decay/) - separate scoring impact report
- [`forgeplan renew`](/docs/cli/renew/) - extend valid_until for still-valid decisions
- [`forgeplan reopen`](/docs/cli/reopen/) - replace stale with a new draft (lineage)
- [`forgeplan deprecate`](/docs/cli/deprecate/) - terminal state for no-longer-valid decisions
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan stale --help`.

```text
Detect stale artifacts with expired valid_until

Usage: forgeplan stale [OPTIONS]

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
