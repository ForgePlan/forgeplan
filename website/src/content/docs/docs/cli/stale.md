---
title: forgeplan stale
description: "List artifacts whose valid_until date has expired. This report does not itself renew them or rewrite lifecycle status. decay separately reports the current effect of expired linked evidence; neither command forecasts future expiry."
---

List artifacts whose valid_until date has expired. This report does not itself renew them or rewrite lifecycle status. `decay` separately reports the current effect of expired linked evidence; neither command forecasts future expiry.

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

```bash
forgeplan stale
forgeplan stale --json | jq '.stale[] | select(.days_expired > 30)'
# After reviewing the selected stale decision:
forgeplan renew ADR-002 --reason "still valid after review" --until 2027-10-01
forgeplan reopen PRD-007 --reason "replace with a new approach"
```

## Choosing an action

Use `renew` after confirming the decision is still valid, `reopen` for a new draft with lineage, or `deprecate` when it no longer applies. Inspect linked evidence separately: expired evidence contributes 0.1 to scoring, but an expiry listing is not a statement that every listed artifact has R_eff 0.1.

## JSON

The envelope contains `stale` and `_next_action`; this CLI response has no `hints` array. Entries contain `id`, `title`, `valid_until`, and `days_expired`. A missing valid_until is not an expired date; do not assume every captured Note automatically expires.

The store includes any record whose parsed date is earlier than today in UTC, without a lifecycle-status filter, and sorts by `valid_until`. RFC 3339 dates can qualify, but the CLI `days_expired` calculation only parses `YYYY-MM-DD` and reports 0 for other accepted formats. Review before following the renewal hint: `renew`'s intended stale-only policy is not fully enforced in 0.38.0 (#547).

## See also

- [`forgeplan decay`](/docs/cli/decay/) - report the current impact of expired evidence
- [`forgeplan renew`](/docs/cli/renew/) - extend valid_until for still-valid decisions
- [`forgeplan reopen`](/docs/cli/reopen/) - replace stale with a new draft (lineage)
- [`forgeplan deprecate`](/docs/cli/deprecate/) - terminal state for no-longer-valid decisions
- [CLI overview](/docs/cli/)
