---
id: DM-{NNN}
title: "{title}"
status: draft
kind: domain_model
tier: factum
created: YYYY-MM-DD
updated: YYYY-MM-DD
---

# DM-{NNN}: {Title}

## Domain
What bounded context / subdomain does this model cover?

## Composition
This model aggregates:
- GLOS-XXX — canonical terms (ubiquitous language)
- INV-XXX — invariants (business rules)
- UC-XXX — use cases (capabilities)
- SCEN-XXX — scenarios (examples)
- HYP-XXX — open hypotheses (where we're still inferring)

## Canonical Forms (rendered)
- DDL — relational schema (if the model has a persistence concern)
- SDL — GraphQL / API surface (if the model exposes a contract)
- Pseudo-code — algorithm sketch (if the model encodes a transformation)

Canonical rendering is a planned Phase E integration in marketplace#79
(`forgeplan_render_canonical`), not a tool shipped in this target.
Fill the applicable examples below from reviewed sources; no automatic
rendering is implied.

### DDL
```sql
-- replace with source-backed DDL if applicable
```

### SDL
```graphql
# replace with source-backed SDL if applicable
```

### Pseudo-code
```text
# replace with source-backed pseudo-code if applicable
```

## Coverage
- Glossary coverage: N of M canonical terms documented
- Invariant coverage: N of M business rules captured
- Use case coverage: N of M capabilities documented
- Scenario coverage: N scenarios per use case (target ≥ 2)
- Hypothesis status: verified / inferred / parked / refuted counts

(`forgeplan_coverage_business` returns a report; it does not fill this body.)

## Source
Where in the codebase did extraction start? Which Discover Agent passes
produced this model?
