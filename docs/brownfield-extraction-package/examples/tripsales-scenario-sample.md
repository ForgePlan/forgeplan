---
kind: scenario
id: SC-042
feature: "Order confirmation by forwarder"
use_case_ref: UC-003
invariants_verified: [INV-003, INV-005, INV-012]
scenario_type: "happy_path"
gherkin_feature: |
  Feature: Order confirmation by forwarder
    As a forwarder employee
    I want to confirm an order I have committed to
    So that cargo initialization proceeds and the order can advance toward fulfillment

    Background:
      Given an order O1 with status 'accepted'
      And quote Q1 was accepted to create O1
      And company C_forwarder is the forwarder on O1
      And company C_cargo_owner is the cargo_owner on O1
      And user U1 is an employee of C_forwarder
      And user U3 is an employee of C_cargo_owner
      And user U2 is an employee of company C_other that is not a participant of O1

    Scenario: Happy path — forwarder confirms accepted order
      When U1 calls orders_Confirm(O1.id)
      Then O1.status becomes 'forwarder_confirmed'
      And cargo is initialized with available_at derived from O1.form.pickup_date
      And a cargo markings job is enqueued (attempts=50, backoff=10s)
      And the Elasticsearch order index is updated
      And event OrderConfirmedByForwarder is emitted
      And the result includes the updated order with sales_order attached

    Scenario: Non-participant is rejected with NOT_FOUND
      When U2 calls orders_Confirm(O1.id)
      Then an error '__NOT_FOUND__' is raised
      And O1.status remains 'accepted'
      And no cargo entity is created
      And no markings job is enqueued
      And no event is emitted

    Scenario: Cargo_owner mirror path
      When U3 calls orders_Confirm(O1.id)
      Then O1.status becomes 'cargo_owner_confirmed'
      And no cargo entity is created
      And no markings job is enqueued
      And the Elasticsearch order index is updated
      And event OrderConfirmedByCargoOwner is emitted

    Scenario: Transactional rollback on cargo init failure
      Given cargo initialization will fail for O1 (simulated DB error)
      When U1 calls orders_Confirm(O1.id)
      Then the entire operation rolls back
      And O1.status remains 'accepted'
      And no cargo entity is created
      And no markings job is enqueued
      And the error is propagated to the caller
visualizations:
  - type: "mermaid-sequence"
    content: |
      sequenceDiagram
          autonumber
          participant FE as Forwarder UI
          participant GW as Gateway
          participant O as orders service
          participant DBO as orders DB
          participant DBSO as sales-orders DB
          participant Q as Queue
          participant ES as Elasticsearch

          FE->>GW: mutation orders_Confirm(O1.id)
          GW->>O: confirm(id, caller=U1)
          O->>DBO: load order O1
          DBO-->>O: Order{status:'accepted', forwarder_id:C_forwarder.id,...}
          O->>O: assert caller participation (INV-003)
          O->>DBSO: load related sales_order
          DBSO-->>O: SalesOrder{...}
          O->>DBO: update status → 'forwarder_confirmed' (INV-005)
          O->>DBO: initialize cargo (INV-012)
          O->>Q: enqueue markings job {attempts:50, backoff:10s}
          O->>ES: update order index
          O->>O: emit OrderConfirmedByForwarder
          O-->>GW: Order with sales_order
          GW-->>FE: Orders_Order
  - type: "mermaid-state"
    content: |
      stateDiagram-v2
          [*] --> created: quote acceptance?
          created --> accepted: UC-002
          accepted --> forwarder_confirmed: UC-003 (forwarder path)
          accepted --> cargo_owner_confirmed: UC-003 (cargo_owner path)
          forwarder_confirmed --> fulfillment: UC-007 (after cargo_owner also confirms)
          cargo_owner_confirmed --> fulfillment: UC-007 (after forwarder also confirms)
          fulfillment --> completed: UC-008
verification:
  automated: "pending — test spec to be generated from this scenario"
  manual: "last manual walkthrough 2026-04-21 by operator on staging; all 4 scenarios passed"
  source: "inferred_from_code"
  confidence: "inferred"
  evidence_refs: ["EVID-018", "EVID-024", "EVID-027"]
  last_verified: "2026-04-21"
bounded_context: "orders"
lifecycle_state: "draft"
---

# SC-042 — Forwarder confirms an accepted order

## Feature overview
Validates that an order participant (forwarder or cargo_owner) can confirm an accepted order, and that the side-effects differ by party. Non-participants see a NOT_FOUND error to prevent information leakage.

## Gherkin feature

See frontmatter `gherkin_feature` above. The feature has 4 scenarios:
1. Happy path — forwarder confirms (primary).
2. Non-participant is rejected.
3. Cargo_owner mirror path.
4. Transactional rollback on cargo init failure.

## Mermaid sequence (happy path)

See frontmatter visualization 1.

## Mermaid state (order lifecycle around confirm)

See frontmatter visualization 2.

## Verification
- **Automated**: pending. Test spec to be generated from this scenario (planned for Wave 4).
- **Manual**: last run 2026-04-21; all four scenarios pass on staging.
- **Confidence**: `inferred`. Waiting on Domain Owner confirmation of dual-confirm semantics (HYP-042).

## Traceability
- Use case: [UC-003](../use-cases.md#uc-003).
- Invariants:
  - [INV-003](../invariants.md#inv-003) — participant-only visibility.
  - [INV-005](../invariants.md#inv-005) — status transition rules.
  - [INV-012](../invariants.md#inv-012) — cargo initialization on forwarder confirm.
- Terms:
  - [forwarder_confirmed](../glossary.md#forwarder_confirmed) (TERM-012).
  - [cargo_owner_confirmed](../glossary.md#cargo_owner_confirmed) (TERM-013).
  - [forwarder](../glossary.md#forwarder) (TERM-005).
- Related scenarios:
  - SC-043 (cancel an order in forwarder_confirmed with cascade).
  - SC-051 (attempt confirm on a non-confirmable status).
  - SC-052 (start trip — successor use case).
