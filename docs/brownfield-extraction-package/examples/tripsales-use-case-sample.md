---
kind: use-case
id: UC-003
name: "Confirm an order"
actor: "forwarder_employee OR cargo_owner_employee"
trigger:
  type: "user_action"
  interface: "graphql:mutation"
  identifier: "orders_Confirm"
preconditions:
  - "an order exists (INV-001)"
  - "the caller is authenticated (INV-002)"
  - "the caller's company is a participant in the order (INV-003)"
  - "the order is in a status where confirmation is meaningful (INV-005)"
outcome:
  primary: "order status is advanced toward fulfillment commitment; side-effects depend on which party confirmed"
  alternatives:
    - "if caller is cargo_owner employee → status becomes 'cargo_owner_confirmed', no cargo initialization"
    - "if caller is not a participant → __NOT_FOUND__ error, no state change"
steps:
  - step: 1
    actor: "order_participant_employee"
    action: "submits orders_Confirm(order_id) mutation"
    system_response: "gateway forwards to orders service"
  - step: 2
    actor: "system"
    action: "loads the order by id"
    system_response: "throws __NOT_FOUND__ if missing"
  - step: 3
    actor: "system"
    action: "verifies caller participation"
    system_response: "throws __NOT_FOUND__ if caller company is not cargo_owner/forwarder/operator"
  - step: 4
    actor: "system"
    action: "loads the related sales_order"
    system_response: "used to attach to the returned order"
  - step: 5
    actor: "system"
    action: "branches by caller.company.type_company"
    system_response: "forwarder path OR cargo_owner path"
  - step: 6a
    actor: "system (forwarder path)"
    action: "updates order.status to forwarder_confirmed, initializes cargo from order.form, enqueues cargo markings job (attempts=50, backoff=10s), updates ES index"
    system_response: "emits OrderConfirmedByForwarder event"
  - step: 6b
    actor: "system (cargo_owner path)"
    action: "updates order.status to cargo_owner_confirmed, updates ES index"
    system_response: "emits OrderConfirmedByCargoOwner event"
  - step: 7
    actor: "system"
    action: "returns updated order with attached sales_order"
    system_response: "GraphQL response delivered to UI"
invariants_invoked: ["INV-001", "INV-002", "INV-003", "INV-005", "INV-012"]
domain_events_emitted: ["OrderConfirmedByForwarder", "OrderConfirmedByCargoOwner"]
related_use_cases: ["UC-002", "UC-007", "UC-009"]
bounded_context: "orders"
code_refs:
  - "services/v5.orders.service.js:_confirm"
verification:
  source: "inferred_from_code"
  confidence: "inferred"
  evidence_refs: ["EVID-018", "EVID-024"]
  last_verified: "2026-04-21"
lifecycle_state: "draft"
---

# UC-003 — Confirm an order

## Overview
An order participant (either the forwarder who will execute transport, or the cargo_owner whose goods are being moved) confirms their commitment to the order. When both parties have confirmed, a subsequent `trips.start` call can advance the order to `fulfillment`.

## Actors
- **Primary**: forwarder employee (in the happy path) or cargo_owner employee (in the mirror path).
- **Observer**: the other participating company is notified via Orders_OnUpdate subscription.

## Trigger
GraphQL mutation `orders_Confirm(id: Int!)`. Typically invoked from a UI button labelled "Confirm order" or "Подтвердить заказ" on the order detail screen.

## Preconditions
1. Order exists ([INV-001](../invariants.md#inv-001)).
2. Caller is authenticated ([INV-002](../invariants.md#inv-002)).
3. Caller's company is in `{order.cargo_owner_id, order.forwarder_id, order.operator_id}` ([INV-003](../invariants.md#inv-003)).
4. Order is in a state where confirmation can still advance it ([INV-005](../invariants.md#inv-005)).

## Main flow (forwarder path — happy)
1. Forwarder employee submits `orders_Confirm(order_id)`.
2. System loads the order.
3. System checks participation; proceeds.
4. System loads the related sales_order.
5. System sets `order.status = 'forwarder_confirmed'`.
6. System initializes cargo entity with `available_at` derived from `order.form.pickup_date` ([INV-012](../invariants.md#inv-012)).
7. System enqueues a cargo markings generation job (attempts=50, exponential backoff starting at 10s).
8. System updates the Elasticsearch search index.
9. System emits `OrderConfirmedByForwarder` domain event.
10. System returns the updated order with `sales_order` attached.

## Alternative flows

### Cargo_owner mirror path
Same as above but step 5 sets status to `cargo_owner_confirmed`; steps 6-7 (cargo init + markings job) are skipped. Step 9 emits `OrderConfirmedByCargoOwner` instead.

### Non-participant rejected
- **Trigger**: caller.company_id not in `{order.cargo_owner_id, order.forwarder_id, order.operator_id}`.
- **Outcome**: error `__NOT_FOUND__` raised. No state change. Intentionally indistinguishable from a non-existing order to avoid leaking existence (see INV-003 rationale).

### Cargo initialization failure
- **Trigger**: database error during cargo init or markings job enqueue.
- **Outcome**: the entire operation rolls back (transactional wrapper). Order status reverts. Error surfaced to caller.

## Outcome
- Order status updated to forwarder_confirmed or cargo_owner_confirmed.
- (Forwarder path only) Cargo entity created; markings job enqueued.
- Elasticsearch order index updated.
- Domain event emitted.

## Domain events
- `OrderConfirmedByForwarder` — payload: `{ order_id, forwarder_id, timestamp }`.
- `OrderConfirmedByCargoOwner` — payload: `{ order_id, cargo_owner_id, timestamp }`.

## Business rules applied
- [INV-001](../invariants.md#inv-001) — order must exist.
- [INV-002](../invariants.md#inv-002) — caller must be authenticated.
- [INV-003](../invariants.md#inv-003) — authorization: participant-only.
- [INV-005](../invariants.md#inv-005) — status transition rules.
- [INV-012](../invariants.md#inv-012) — cargo initialized only on forwarder confirm.

## Related use cases
- [UC-002 Accept a quote](../use-cases.md#uc-002) — creates the order with initial status (precursor).
- [UC-007 Start a trip](../use-cases.md#uc-007) — requires both confirmations to transition to fulfillment (successor).
- [UC-009 Cancel an order](../use-cases.md#uc-009) — can be invoked from forwarder_confirmed with cascade behavior (alternative path out).

## Open questions
<!-- confidence:speculation -->
- Is the dual-confirm sequence order-sensitive? (Does forwarder-first vs cargo_owner-first produce different downstream behavior?)
- What happens if the same user confirms twice? (Parked in IP-2026-04-21 Q4.)
- How does the operator role interact with confirmation? (Parked in IP-2026-04-21 Q5.)
<!-- /confidence -->

See parked hypothesis HYP-042 for candidate interpretations.
