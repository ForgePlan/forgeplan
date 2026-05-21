---
id: SCEN-{NNN}
title: "{title}"
status: draft
kind: scenario
tier: factum
created: YYYY-MM-DD
updated: YYYY-MM-DD
---

# SCEN-{NNN}: {Title}

## Given (preconditions)
Concrete starting state. Real values, not abstractions.

## When (action)
What happens? A specific event or action triggered by an actor.

## Then (outcome)
What should be observed afterwards? Concrete, verifiable outcome.

## Demonstrates
- INV-XXX — invariant this scenario exercises
- UC-XXX — use case this scenario is an example of

## Probes Boundary?
Is this scenario the "happy path" example or does it deliberately probe
an edge case / boundary condition? If boundary — what's the boundary?

## Counter-example
If this scenario is interesting because it demonstrates what does NOT
happen — describe the negation here.

## Source
Where in the codebase / docs was this scenario extracted (test fixture,
real bug report, hypothetical from interviews)?
