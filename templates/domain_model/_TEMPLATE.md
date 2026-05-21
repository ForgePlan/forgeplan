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

These come from `forgeplan_render_canonical` (Phase E in marketplace#79);
placeholders below get filled by that pipeline.

### DDL
```sql
-- placeholder; rendered by render_canonical
```

### SDL
```graphql
# placeholder; rendered by render_canonical
```

### Pseudo-code
```text
# placeholder; rendered by render_canonical
```

## Coverage
- Glossary coverage: N of M canonical terms documented
- Invariant coverage: N of M business rules captured
- Use case coverage: N of M capabilities documented
- Scenario coverage: N scenarios per use case (target ≥ 2)
- Hypothesis status: verified / inferred / parked / refuted counts

(filled by `forgeplan_coverage_business` — Phase C)

## Source
Where in the codebase did extraction start? Which Discover Agent passes
produced this model?
