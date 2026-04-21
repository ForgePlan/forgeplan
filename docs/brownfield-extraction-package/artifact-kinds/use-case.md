# Artifact kind: `use-case`

> A user journey from a triggering event to a business outcome. Captures the **flow**, not the code.

## Purpose

Use cases describe **what the business does** through the system, at a level that is meaningful to non-developers. They are the unit of scope for scenarios, the anchor for invariants, and the primary entity in business documentation.

A use case is NOT a function, action, or endpoint. It's a cross-cutting journey that may span multiple services.

## When to create

- For each distinct business flow from an actor's intent to a completion state.
- When an entry point (GraphQL mutation, REST endpoint, queue consumer, scheduled job) triggers a meaningful business operation.
- When a scenario needs to be written.

## Frontmatter schema

```yaml
kind: use-case
id: UC-{auto}
name: "Forwarder confirms an order"
actor: "forwarder_employee"
trigger:
  type: "user_action" | "event" | "schedule"
  interface: "graphql:mutation" | "rest:endpoint" | "queue:name" | "cron:schedule"
  identifier: "orders_Confirm"
preconditions:
  - "an order exists in status 'accepted'"
  - "the caller is an employee of the order's forwarder company"
outcome:
  primary: "order status becomes 'forwarder_confirmed'; cargo is initialized; markings generation is enqueued"
  alternatives:
    - "if caller is cargo_owner employee → status becomes 'cargo_owner_confirmed' instead"
steps:
  - step: 1
    actor: "forwarder_employee"
    action: "submits confirm request with order ID"
    system_response: "loads order, validates caller is a participant"
  - step: 2
    actor: "system"
    action: "determines caller company type"
    system_response: "branches to forwarder or cargo_owner path"
  - step: 3
    actor: "system"
    action: "updates order status, initializes cargo, enqueues markings job"
    system_response: "persists, publishes event, returns updated order"
invariants_invoked: ["INV-003", "INV-005", "INV-012"]
domain_events_emitted: ["OrderConfirmedByForwarder"]
related_use_cases: ["UC-002", "UC-007"]
bounded_context: "orders"
code_refs:          # ONLY used by Tier 1 factum layer; NEVER appears in canonical output
  - "services/v5.orders.service.js:_confirm"
verification:
  source: "inferred_from_code" | "domain_owner" | "external_doc"
  confidence: "verified" | "strong-inferred" | "inferred" | "speculation"
  evidence_refs: ["EVID-019"]
  last_verified: "2026-04-21"
lifecycle_state: "active" | "deprecated" | "superseded"
```

## Body structure

1. **Overview** (2-3 sentences — what this use case is for, at business level).
2. **Actors** (who triggers, who receives, who observes).
3. **Trigger** (entry point — named but with business framing).
4. **Preconditions** (what must be true before; reference invariants).
5. **Main flow** (numbered steps, actor/action/result).
6. **Alternative flows** (branches and failures).
7. **Outcome** (post-conditions; reference invariants).
8. **Domain events** (bus events emitted).
9. **Business rules applied** (reference to invariants).
10. **Related use cases** (prerequisites, variations, successors).
11. **Open questions** (things that need Domain Owner input).

## Example (TripSales)

```markdown
---
kind: use-case
id: UC-003
name: "Confirm an order"
actor: "forwarder_employee OR cargo_owner_employee"
trigger:
  type: "user_action"
  interface: "graphql:mutation"
  identifier: "orders_Confirm"
# ...
---

# UC-003 — Confirm an order

## Overview
An order participant (either the forwarder who will execute transport, or the cargo_owner whose goods are being moved) confirms their commitment to the order. When both parties have confirmed, the order transitions to fulfillment.

## Actors
- **Primary**: forwarder employee (happy path) or cargo_owner employee (mirror path).
- **Observer**: the other party (notified via subscription).

## Trigger
Mutation `orders_Confirm(id)`.

## Preconditions
1. Order exists (INV-001).
2. Caller is authenticated (INV-002).
3. Caller's company is one of the order's participants (INV-003).
4. Order status is one of the states where confirmation is meaningful (INV-005).

## Main flow (forwarder path)
1. Forwarder employee submits `orders_Confirm(order_id)`.
2. System loads the order.
3. System checks caller participation (INV-003).
4. System loads related sales_order.
5. System sets order.status = 'forwarder_confirmed'.
6. System initializes cargo from order.form (see cargo initialization rules).
7. System enqueues a cargo markings generation job (retry 50x, backoff 10s).
8. System updates the Elasticsearch search index.
9. System returns the updated order with sales_order.

## Alternative flows

### Cargo_owner path (mirror)
Same as above, but step 5 sets status = 'cargo_owner_confirmed' and steps 6-7 are skipped.

### Non-participant rejected
- Trigger: caller.company not in {cargo_owner_id, forwarder_id, operator_id}.
- Outcome: __NOT_FOUND__ error raised; no state change.

### Cargo init failure
- If cargo initialization fails, the entire operation rolls back (transactional).

## Outcome
- Order status updated.
- Cargo entity created (forwarder path only).
- Markings job enqueued (forwarder path only).
- Elasticsearch index updated.
- Domain event `OrderConfirmedByForwarder` or `OrderConfirmedByCargoOwner` emitted.

## Domain events
- `OrderConfirmedByForwarder` (forwarder path).
- `OrderConfirmedByCargoOwner` (cargo_owner path).

## Business rules applied
- INV-003 (authorization).
- INV-005 (status transition rules).
- INV-012 (cargo is only initialized on forwarder confirmation).

## Related use cases
- UC-002 — Accept a quote (precursor; creates the order).
- UC-007 — Start a trip (successor; only when both confirms present).

## Open questions
- Is `rejected` status reachable through confirm? (parked in interview packet; see IP-2026-04-21).
```

## Validation rules

- `actor` must be a role (not a username).
- `trigger.identifier` must exist (parsed from code or verified).
- `steps` must have at least 1 entry.
- `invariants_invoked` must reference existing `invariant` IDs.
- `verification.confidence` = `speculation` requires the body to wrap speculative parts in `<!-- confidence:speculation -->...<!-- /confidence -->`.

## Links

- `triggers` — (use-case) triggers (domain-event).
- `verifies` — (scenario) verifies (use-case).
- `informs` — (use-case) informs (domain-model).
- `based_on` — (use-case) based_on (glossary) for terms used.

## Lifecycle

Same as `glossary`.

## Freshness

Re-validate when the trigger's code is modified (detected via `forgeplan_drift` tied to the symbol).
