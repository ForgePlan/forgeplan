---
title: forgeplan renew
description: "Extend the validity of a stale artifact and bring it back to active - the 'still valid' answer to valid_until expiry."
---

`forgeplan renew` transitions a `stale` artifact back to `active` by setting a new `valid_until` date and recording a reason for the extension. valid_until is optional, not automatic for every scored artifact. Expiration is surfaced by stale/health reports; renewal is for a reviewed artifact whose stored lifecycle state is stale. Renew is the fast path when a review confirms the decision is still correct - no rewrite, no successor, just an extended validity window and a note explaining why the team trusts it for another cycle.

## Forgeplan 0.38.0

Use renew for a re-reviewed stale decision, not instead of activate on a draft. The 0.38.0 implementation validates date format and the generic transition to active; it does not separately require a future date or strictly stale source status. This is an implementation limitation, not a quality gate. Choose a meaningful future date yourself. The command appends a Renewal section and updates status and valid_until.

## When to use

- An ADR's `valid_until` expired, but a quarterly architecture review confirmed the decision still holds - push the expiry forward with a reason referencing the review.
- A PRD went stale during a slow sprint but the requirements are unchanged and still accurate.
- An EvidencePack's measurement window elapsed, but a re-run of the same benchmark produced equivalent numbers.
- A RefreshReport concluded "no changes needed" and you want the source artifact to reflect that outcome.

## When NOT to use

- The decision has actually changed - draft a successor and use [`forgeplan supersede`](/docs/cli/supersede/) instead.
- You want to re-evaluate the problem from scratch - use [`forgeplan reopen`](/docs/cli/reopen/) to deprecate the old artifact and start a new draft.
- The artifact is no longer relevant at all - use [`forgeplan deprecate`](/docs/cli/deprecate/).
- The artifact is still `active` — the generic state machine refuses active → active rather than silently extending it.

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

### Example 1: Renew an ADR after an architecture review

```bash
forgeplan renew ADR-001 --reason "still valid after Q2 architecture review, no changes" --until 2027-10-01
```

The canonical use case: a review confirmed the decision stands, so bump the expiry and log the evidence of that review in the reason.

### Example 2: Find and renew all stale artifacts you want to keep

```bash
forgeplan stale
forgeplan renew PRD-004 --reason "requirements unchanged, confirmed with product" --until 2026-12-31
forgeplan renew RFC-006 --reason "implementation phases still on track" --until 2027-09-15
```

Run `forgeplan stale` to find expired work, then inspect its stored status with `get`. Expiration alone does not persist stale. For a reviewed, expired active decision, explicitly mark it stale with `forgeplan update ID --status stale` before renewal; substitute its actual ID. Do not apply this workflow to a draft or terminal artifact.

### Example 3: Short-horizon renewal while a follow-up is drafted

```bash
forgeplan renew ADR-007 --reason "temporary extension until RFC-018 is ratified" --until 2027-05-01
```

Useful bridge when you know a replacement is coming but the current decision must remain authoritative in the meantime.

## How it fits the workflow

Renew is part of the decision-maintenance loop that runs alongside the main `Shape → Validate → Code → Evidence → Activate` cycle. For artifacts with valid_until, stale/health surface expiration for review; a report is not a background status-transition daemon. For each stale artifact, the team chooses: renew (still valid), supersede (replaced), deprecate (retired), or reopen (re-evaluate). Renew is the lowest-cost option - it keeps the original body and appends a Renewal section containing the reason, while updating status and valid_until.

## Common errors

| Error | Cause | Fix |
|---|---|---|
| `Invalid date format` | Wrong format (e.g. `10/01/2026`) | Use ISO: `2026-10-01` |
| `Invalid transition` | Source status is active or terminal | Renew a reviewed artifact whose stored status is stale; create a separate new draft for terminal work. |

## See also

- [`forgeplan stale`](/docs/cli/stale/) - list artifacts with expired `valid_until`
- [`forgeplan reopen`](/docs/cli/reopen/) - if the decision needs re-evaluation from scratch
- [`forgeplan deprecate`](/docs/cli/deprecate/) - if the decision no longer applies
- [`forgeplan supersede`](/docs/cli/supersede/) - if a replacement already exists
- [`forgeplan health`](/docs/cli/health/) - surfaces stale artifacts that need attention
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Methodology: Artifact Lifecycle](/docs/methodology/lifecycle/)
