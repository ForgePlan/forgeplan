---
id: HYP-{NNN}
title: "{title}"
status: draft
kind: hypothesis
tier: intent
hypothesis_state: inferred
created: YYYY-MM-DD
updated: YYYY-MM-DD
---

# HYP-{NNN}: {Title}

## Hypothesis
A specific, falsifiable claim about how the system / domain works.
State it as a sentence that can be either confirmed or refuted by
evidence.

## Lifecycle State
**Current**: `inferred`

Allowed transitions:
- `parked` → `inferred` (resume work)
- `inferred` → `strong-inferred` (corroborating evidence)
- `strong-inferred` → `verified` (sufficient evidence, terminal)
- any → `refuted` (counter-evidence, terminal)
- any → `parked` (insufficient signal, hold)

## Confidence
Why we believe this hypothesis now: signals that point at it.

## Evidence For
- EVID-XXX — supporting evidence
- file:line — direct code reference

## Evidence Against
- file:line — code that contradicts
- Counter-scenarios that would falsify the hypothesis

## How To Verify
What experiment / test / interview / code-read would move this from
`inferred` to either `verified` or `refuted`?

## Cascade
If this hypothesis is verified / refuted, which downstream artifacts
must be revisited?
- INV-XXX, UC-XXX, SCEN-XXX

## Source
What observation triggered this hypothesis (extraction pass, contradiction,
discrepancy with docs)?
