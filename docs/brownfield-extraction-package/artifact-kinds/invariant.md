# Artifact kind: `invariant`

> A business rule that must hold at all times (or at specified points). Captures the **why**, not just the **what**.

## Purpose

Invariants are the **rules of the business universe**. Violating one means the business has entered an impossible state. They are the atomic unit of correctness against which all scenarios and use-cases are verified.

Code guards (`if … throw`) are the *implementation* of invariants. The invariant itself is the business-level rule the guard protects.

## When to create

- When a code guard (`if … throw`, `assert`, schema `NOT NULL`, FK constraint) encodes a business rule.
- When an interview reveals a business rule not yet captured.
- When a scenario's Given/Then depends on a rule that must be stated.

## Frontmatter schema

```yaml
kind: invariant
id: INV-{auto}
statement: "Orders are visible only to their participants (cargo_owner, forwarder, or operator of the order)."
domain: "orders"
category: "authorization" | "state_transition" | "referential_integrity" | "temporal" | "financial" | "data_validation"
scope: "always" | "precondition" | "postcondition" | "event_handling"
code_guards:     # ONLY Tier 1; NEVER in canonical output
  - path: "services/v5.orders.service.js:_assertParticipant"
    snippet: "if (!user || ![oco, ofw, oop].includes(user.company_id)) throw '__NOT_FOUND__'"
violation_consequence: "Error '__NOT_FOUND__' raised; no state change; caller sees order as if it did not exist."
rationale: "Privacy between business parties; an order's existence leaks competitive info."
related_invariants: ["INV-001", "INV-005"]
affected_use_cases: ["UC-003", "UC-009", "UC-015"]
verification:
  source: "inferred_from_code" | "domain_owner" | "external_doc"
  confidence: "verified" | "strong-inferred" | "inferred" | "speculation"
  evidence_refs: ["EVID-023"]
  last_verified: "2026-04-21"
bounded_context: "orders"
lifecycle_state: "active" | "deprecated" | "superseded"
```

## Categories

| Category | Examples |
|---|---|
| `authorization` | Only participants see orders; only admins edit RBAC rules. |
| `state_transition` | Orders transition `accepted → forwarder_confirmed → fulfillment` only under specified conditions. |
| `referential_integrity` | Every trip references a valid order. |
| `temporal` | `pickup_date` must be after `order.created_at`. |
| `financial` | `payment.amount` must be positive; invoice total must equal sum of items. |
| `data_validation` | `email` must match RFC 5322; `company_inn` must pass checksum. |

## Body structure

1. **Statement** (one plain-English sentence; the rule itself).
2. **Rationale** (why this rule exists — business reason).
3. **Scope** (when it must hold: always, at triggering, at completion).
4. **Violation consequence** (what happens if broken — from business and system perspective).
5. **Covered by** (use cases and scenarios that exercise this invariant).
6. **Related invariants** (stronger/weaker rules, or dependencies).
7. **Contradictions** (other invariants that conflict — must be resolved).
8. **Evidence** (for factum: code paths; for intent: interviews / docs).

## Example (TripSales)

```markdown
---
kind: invariant
id: INV-003
statement: "An order is visible only to employees of its participating companies (cargo_owner, forwarder, or operator)."
domain: "orders"
category: "authorization"
scope: "always"
verification:
  source: "inferred_from_code"
  confidence: "strong-inferred"
  evidence_refs: ["EVID-011"]
  last_verified: "2026-04-21"
---

# INV-003 — Order visibility limited to participants

## Statement
An order is visible only to employees whose company is one of the order's `cargo_owner_id`, `forwarder_id`, or `operator_id`.

## Rationale
Orders contain sensitive commercial information (rates, route details, cargo value). Exposing an order to non-participants would leak competitive intelligence and violate participant agreements. The business requires strict partitioning between parties.

## Scope
Always. Any action that reads or modifies an order must first verify the caller is a participant.

## Violation consequence
- System: error `__NOT_FOUND__` (intentionally indistinguishable from a non-existing order — avoids confirming the order's existence to an unauthorized caller).
- Business: caller cannot discover what orders exist outside their sphere.

## Covered by
- UC-003 (confirm)
- UC-009 (cancel)
- UC-015 (get by id)
- Scenarios: SC-042, SC-043, SC-051

## Related invariants
- INV-001 (orders exist with a valid ID) — prerequisite.
- INV-008 (operator can see orders where they are operator only) — narrower rule in same family.

## Contradictions
None known.

## Evidence
- Code guard: see EVID-011.
- Multiple call sites confirm the pattern.

Hypothesis (Domain Owner to verify): is an admin user (e.g. TripSales staff) ALSO granted visibility? Current code suggests no, but `rbac.view` rules exist that might override. Confidence: inferred.
```

## Validation rules

- `statement` must be one sentence, plain English (or Russian).
- `category` must match the list above.
- `affected_use_cases` must reference existing `use-case` IDs.
- If `category = contradicts` relation exists → must be resolved before state `active`.

## Links

- `verifies` — (scenario) verifies (invariant).
- `applies_to` — (invariant) applies_to (use-case).
- `defines` — (invariant) defines (domain-model) constraint.
- `contradicts` — (invariant) contradicts (invariant).
- `refines` — (invariant) refines (invariant) narrower case.

## Lifecycle

- `draft` — newly proposed.
- `active` — verified or strong-inferred; applied in validation.
- `deprecated` — rule no longer enforced (e.g. business changed).
- `superseded` — replaced by a narrower/broader version.

## Freshness

Re-validate when the underlying code guard changes. Drift on the guard triggers a re-verification task.
