---
title: forgeplan renew
description: "Extend the validity of a stale artifact and bring it back to active - the 'still valid' answer to valid_until expiry."
---

`forgeplan renew` appends a `Renewal` reason section, writes `status: active` and updates `valid_until`. The source artifact may have no `valid_until`, but this command requires `--until <YYYY-MM-DD>`. An overdue date does not itself persist `status: stale`. The stored-status guard permits stale→active and draft→active, not active→active. Review evidence before extending a date; renewal itself is not a new measurement.

## When to use

- An ADR's `valid_until` expired, but a quarterly architecture review confirmed the decision still holds - push the expiry forward with a reason referencing the review.
- A PRD went stale during a slow sprint but the requirements are unchanged and still accurate.
- An EvidencePack's measurement window elapsed, but a re-run of the same benchmark produced equivalent numbers.
- A RefreshReport concluded "no changes needed" and you want the source artifact to reflect that outcome.

## When NOT to use

- The decision has actually changed - draft a successor and use [`forgeplan supersede`](/docs/cli/supersede/) instead.
- You want to re-evaluate the problem from scratch - use [`forgeplan reopen`](/docs/cli/reopen/) to deprecate the old artifact and start a new draft.
- The artifact is no longer relevant at all - use [`forgeplan deprecate`](/docs/cli/deprecate/).
- Stored status is `active` - an overdue date alone does not make active→active renewal legal.

## Usage

```text
forgeplan renew --reason <REASON> --until <UNTIL> <ID>
```

## Arguments

```text
  <ID>  Artifact ID
```

## Options

```text
      --reason <REASON>  Reason for renewal
      --until <UNTIL>    New valid_until date (YYYY-MM-DD)
  -h, --help             Print help
  -V, --version          Print version
```

## Examples

Dates and IDs below are illustrative. Choose a future date justified by your
review; historical sample dates are not current recommended deadlines.

### Example 1: Renew an ADR after an architecture review

```bash
forgeplan renew ADR-001 --reason "still valid after Q2 architecture review, no changes" --until 2026-10-01
```

The canonical use case: a review confirmed the decision stands, so bump the expiry and log the evidence of that review in the reason.

### Example 2: Find and renew all stale artifacts you want to keep

```bash
forgeplan stale
forgeplan renew PRD-004 --reason "requirements unchanged, confirmed with product" --until 2026-12-31
forgeplan renew RFC-006 --reason "implementation phases still on track" --until 2026-09-15
```

Run `forgeplan stale` to see the full backlog of expired artifacts, then renew the ones that pass review.

### Example 3: Short-horizon renewal while a follow-up is drafted

```bash
forgeplan renew ADR-007 --reason "temporary extension until RFC-018 is ratified" --until 2026-05-01
```

Useful bridge when you know a replacement is coming but the current decision must remain authoritative in the meantime.

## How it fits the workflow

Renew is part of decision maintenance. `stale` detects elapsed dates without changing source status. Review both the date and stored status. Renewal preserves existing body content and appends a reason, so it is not metadata-only.

## Common errors

| Error | Cause | Fix |
|---|---|---|
| Transition refused | Stored status does not permit transition to active | Inspect source status; an overdue date is not a status update |
| `Invalid date format` | Date cannot be parsed | Use `YYYY-MM-DD` |

The selected handler validates format but does not require a future date.
Choose a justified future date yourself; do not rely on a nonexistent future-date check.

## See also

- [`forgeplan stale`](/docs/cli/stale/) - list artifacts with expired `valid_until`
- [`forgeplan reopen`](/docs/cli/reopen/) - if the decision needs re-evaluation, not just extension
- [`forgeplan deprecate`](/docs/cli/deprecate/) - if the decision no longer applies
- [`forgeplan supersede`](/docs/cli/supersede/) - if a replacement already exists
- [`forgeplan health`](/docs/cli/health/) - surfaces stale artifacts that need attention
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Methodology: Artifact Lifecycle](/docs/methodology/lifecycle/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan renew --help`.

```text
Renew a stale artifact (stale → active) with extended validity

Usage: forgeplan renew --reason <REASON> --until <UNTIL> <ID>

Arguments:
  <ID>  Artifact ID

Options:
      --reason <REASON>  Reason for renewal
      --until <UNTIL>    New valid_until date (YYYY-MM-DD)
  -h, --help             Print help
  -V, --version          Print version
```
<!-- forgeplan-runtime:end -->
