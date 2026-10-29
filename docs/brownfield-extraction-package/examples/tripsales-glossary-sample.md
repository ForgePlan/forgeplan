---
kind: glossary
id: TERM-012
term: "forwarder_confirmed"
aliases: ["forwarder confirmation", "FC status"]
domain: "orders"
definition: "Order status indicating that the forwarder employee has committed the order for fulfillment, triggering cargo initialization and markings generation."
code_usage:
  - path: "models/Order.js"
    role: "ENUM value in the status column"
  - path: "services/v5.orders.service.js"
    role: "set by action confirm when caller.company.type_company = 'forwarder'"
  - path: "services/v5.db.orders.service.js"
    role: "accepted value in the update.status action guard"
related_terms: ["TERM-013", "TERM-005", "TERM-001", "TERM-014"]
contradictions: []
verification:
  source: "inferred_from_code"
  confidence: "inferred"
  evidence_refs: ["EVID-018"]
  last_verified: "2026-04-21"
bounded_context: "orders"
lifecycle_state: "draft"
---

# TERM-012 — forwarder_confirmed

Historical extraction example, not a current validated Forgeplan artifact. Original TripSales source/evidence receipts and the other TERM IDs are not shipped here. The confirmation interpretation below is inferred and unresolved.

## Definition
Order status indicating that the forwarder employee has committed the order for fulfillment. Setting this status triggers cargo initialization from the order's form and enqueues a cargo markings generation job.

## Context
One of two confirmation-state values an order can hold:
- `forwarder_confirmed`: applied when the confirming caller is an employee of the forwarder company.
- `cargo_owner_confirmed`: applied when the confirming caller is an employee of the cargo_owner company.

Hypothesis: both confirmations may be required before `fulfillment`. This is not established here; the package's C9 example raises a conflicting trip-start observation.

## Aliases
- "forwarder confirmation" (appears in UI labels).
- "FC status" (informal shorthand used in comments).

## Examples
- Order O1 with status `accepted`: forwarder employee U1 calls `orders_Confirm(O1.id)` → O1.status becomes `forwarder_confirmed`. Cargo is initialized; markings job enqueued.
- Open example question: after the cargo_owner confirms an order already in `forwarder_confirmed`, which state is stored and what permits `trips.start`? No intermediate state or guard is established by this sample.

## Contradictions / ambiguities
<!-- confidence:speculation -->
The pairing semantics — whether the order must remain strictly in `forwarder_confirmed` OR `cargo_owner_confirmed` before the other party confirms, or whether it can be in either and the second call moves to fulfillment — is not fully resolved from code alone. Parked in [interview packet IP-2026-04-21 Q1](interview-packet-orders-sample.md#q1--dual-confirm-pattern).
<!-- /confidence -->

## Evidence
- EVID-018 — trace of `_confirm` action in `services/v5.orders.service.js` showing the branching on `user.company.type_company`.
- Supporting hypothesis: HYP-042 (selected candidate H1: two-party commitment pattern).

## Related
- Related terms: `cargo_owner_confirmed` (TERM-013), `forwarder` (TERM-005), `order_status` (TERM-001), `fulfillment` (TERM-014).
- Used by use-cases: UC-003 (confirm), UC-007 (start trip), UC-009 (cancel cascade).
- Referenced in invariants: INV-005 (status transition rules), INV-012 (cargo init on forwarder confirm).
