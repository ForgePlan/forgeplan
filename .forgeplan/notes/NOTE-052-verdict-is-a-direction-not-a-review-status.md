---
depth: tactical
id: NOTE-052
kind: note
links:
- target: PRD-086
  relation: informs
status: draft
title: 'verdict: is a direction, not a review status'
---

---
assigned_number: 52
created: 2026-09-07
predicted_number: 52
slug: note-verdict-is-a-direction-not-a-review-status
updated: 2026-09-07
---

# `verdict:` is a direction, not a review status

## What happened

`EVID-136` — a code review of the blog scaffold — declared `verdict: concerns`.
There is no such value. The scorer accepted three: `supports`, `weakens`,
`refutes`.

Until PRD-086 FR-008 the unrecognised value fell through to `supports` and
scored **1.0**, so a review whose own summary read *"three issues require coder
attention before merge"* counted as full confirmation of the thing it objected
to. `PRD-079` and `RFC-011` carried that 1.0 for three weeks.

FR-008 made the fall-through fail closed, which is what surfaced it. Fixed by
setting `weakens`: the review does not refute the PRD, it lowers confidence in
the state reviewed. Scores now read 0.30 / 0.30 / 0.20 — the first honest
numbers those three artifacts have had.

## The trap worth naming

`CONCERNS` is a real word in this project: it is one of the three verdicts a
Profile-B reviewer returns (`PASS` / `CONCERNS` / `BLOCKER`). The author wrote
their review status into a field that wants something else, and both vocabularies
are correct in their own place.

The pack's `## Verdict` heading said `CONCERNS` too, which is right — that is
prose about the review. The structured field beneath it is a different question:
**which way does this evidence push the claim.** A review can be CONCERNS and
still be `weakens`; it could equally be CONCERNS and `supports` if the concerns
are cosmetic.

Rule of thumb when writing an EvidencePack:

- `supports` — the claim is more likely true because of this
- `weakens` — less likely, or true only under narrower conditions
- `refutes` — the claim as stated is wrong

Review verdicts, test outcomes and CI statuses belong in the prose, not in this
field.

## Scale

One pack in 173. The discipline holds — which is exactly why nobody noticed the
one that did not, and why the gate had to exist rather than the convention.

Related: PRD-086 FR-008, PROB-101.


