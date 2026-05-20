---
id: INV-{NNN}
title: "{title}"
status: draft
kind: invariant
tier: factum
created: YYYY-MM-DD
updated: YYYY-MM-DD
---

# INV-{NNN}: {Title}

## Statement
A precise, testable business rule that MUST always hold. Imperative voice.

> Example: "A refund cannot exceed the original payment amount."

## Scope
Where does this invariant apply? Which artifacts / data / use cases?

## Rationale
Why does this rule exist? What goes wrong if it's violated?

## Verification
How can we verify this invariant holds in the codebase?
- [ ] Static check (type system / compiler)
- [ ] Test (unit / integration / property-based)
- [ ] Runtime assertion / guard
- [ ] Manual audit

## Covering Scenarios
- SCEN-XXX — concrete example that demonstrates the rule
- SCEN-XXX — edge case that probes the boundary

## Hypothesis Origin
- HYP-XXX — if this invariant came from an inferred hypothesis, link it

## Source
Where in the codebase / docs was this rule extracted or inferred from?
