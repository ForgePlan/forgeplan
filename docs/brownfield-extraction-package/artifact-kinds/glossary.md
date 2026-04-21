# Artifact kind: `glossary`

> Captures a single business term — its meaning, aliases, usage, and confidence in that meaning.

## Purpose

The glossary is the **foundation layer** of business documentation. Before a use-case, invariant, or scenario can be written unambiguously, every business term used in it must exist in the glossary with an agreed meaning.

Brownfield projects have fragmented language: `forwarder`, `forwarder_id`, `operator_id` may all refer to similar entities with different subtleties. The glossary forces disambiguation.

## When to create

- Every time a new business term appears in code/docs/interviews.
- When a term acquires a new shade of meaning in a specific context.
- When two existing terms are discovered to be synonyms.

## Frontmatter schema

```yaml
kind: glossary
id: TERM-{auto}
term: "forwarder_confirmed"
aliases: ["forwarder confirmation", "экспедитор подтвердил"]
domain: "orders"
definition: "Order status indicating the forwarder employee has committed the order for fulfillment."
code_usage:
  - path: "models/Order.js"
    role: "enum value in status field"
  - path: "services/v5.orders.service.js"
    role: "set after _confirm when caller is forwarder employee"
related_terms: ["cargo_owner_confirmed", "forwarder", "order_status"]
contradictions: []    # list of term IDs that conflict
verification:
  source: "inferred_from_code" | "domain_owner" | "external_doc"
  confidence: "verified" | "strong-inferred" | "inferred" | "speculation"
  evidence_refs: ["EVID-023"]
  last_verified: "2026-04-21"
bounded_context: "orders"
lifecycle_state: "active" | "deprecated" | "superseded"
```

## Body structure

1. **Definition** (1-2 sentences, plain business language).
2. **Context** (when / where / why the term is used).
3. **Aliases** (with domain/source for each).
4. **Examples** (concrete instances).
5. **Contradictions or ambiguities** (if any).
6. **Evidence** (pointers to code, docs, or interviews that support the definition).

## Example (TripSales)

```markdown
---
kind: glossary
id: TERM-012
term: "forwarder_confirmed"
aliases: ["forwarder confirmation"]
domain: "orders"
definition: "Order status indicating the forwarder employee has committed the order for fulfillment, triggering cargo initialization and markings generation."
code_usage:
  - path: "models/Order.js"
    role: "ENUM value in status column"
  - path: "services/v5.orders.service.js"
    role: "set by action 'confirm' when caller.company.type_company='forwarder'"
related_terms: ["cargo_owner_confirmed", "forwarder", "order_status", "fulfillment"]
contradictions: []
verification:
  source: "inferred_from_code"
  confidence: "inferred"
  evidence_refs: ["EVID-018"]
  last_verified: "2026-04-21"
bounded_context: "orders"
lifecycle_state: "active"
---

# forwarder_confirmed

## Definition
Order status indicating that the forwarder employee has committed the order for fulfillment. Triggers cargo initialization from the order's form and enqueues a cargo markings generation job.

## Context
One of two confirmation-state values for an order:
- `forwarder_confirmed`: set when the confirming caller is a forwarder employee.
- `cargo_owner_confirmed`: set when the confirming caller is a cargo_owner employee.

The order must pass through BOTH confirmation states before it can transition to `fulfillment`.

## Aliases
- "forwarder confirmation" (used in UI strings)
- `FC` shorthand in some comments

## Examples
- Order with status=`accepted`, forwarder U1 calls `confirm` → status becomes `forwarder_confirmed`.

## Contradictions
None known.

## Evidence
- `EVID-018` — code trace of confirm action.
- `EVID-023` — scenario test output.

Hypothesis (unverified by Domain Owner): this dual-confirm pattern exists because forwarder commits resources and cargo_owner commits data separately, and both commitments are needed before the goods can move. **Confidence: inferred.**
```

## Validation rules

- `term` unique within `bounded_context`.
- `aliases` must be disjoint across all `glossary` artifacts.
- `related_terms` must reference existing `glossary` IDs.
- `contradictions` must also reference existing `glossary` IDs.
- `verification.confidence` = `speculation` is allowed but must be flagged in the rendered body.

## Links

- `code_refers_to` — link to `code-symbol` nodes in the KG (not stored as forgeplan artifacts, but in the KG).
- `defines` — (glossary) defines terms used by (use-case), (invariant), (scenario).
- `synonymy` — (glossary) synonymy (glossary).
- `contradicts` — (glossary) contradicts (glossary).

## Lifecycle

- `draft` — newly created, not yet triangulated.
- `active` — verified or inferred with confidence.
- `deprecated` — no longer used but kept for historical clarity.
- `superseded` — replaced by another term (`superseded_by` field).

## Freshness

Re-validate every 60 days if code mentions have changed (detected via `forgeplan_drift`).
