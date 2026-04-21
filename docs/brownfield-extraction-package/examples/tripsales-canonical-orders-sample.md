# Canonical Orders Domain (sample render)

> Sample output of skill C10 (`canonical-reproducer`) for the Orders domain. Shows what a self-contained, standalone render looks like — zero `file:line` references.

## canonical/orders/ddl.sql

```sql
-- Standalone DDL for Orders domain
-- Generated from domain-model DM-001; validated by C11 (psql --check passes).

CREATE TYPE order_status_enum AS ENUM (
  'created',
  'accepted',
  'cargo_owner_confirmed',
  'forwarder_confirmed',
  'fulfillment',
  'rejected',
  'canceled',
  'completed',
  'removed'
);

CREATE TABLE orders (
  id SERIAL PRIMARY KEY,
  code VARCHAR(50) NOT NULL UNIQUE,
  status order_status_enum NOT NULL DEFAULT 'created',
  cargo_owner_id INT NOT NULL,
  forwarder_id INT NOT NULL,
  operator_id INT,
  quote_id INT NOT NULL,
  form JSONB NOT NULL DEFAULT '{}'::jsonb,
  paid BOOLEAN NOT NULL DEFAULT FALSE,
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_orders_status ON orders(status);
CREATE INDEX idx_orders_cargo_owner ON orders(cargo_owner_id);
CREATE INDEX idx_orders_forwarder ON orders(forwarder_id);
CREATE INDEX idx_orders_operator ON orders(operator_id) WHERE operator_id IS NOT NULL;

-- Deferred FKs (resolved after dependent domains materialize)
ALTER TABLE orders ADD CONSTRAINT fk_orders_quote
  FOREIGN KEY (quote_id) REFERENCES quotes(id);
ALTER TABLE orders ADD CONSTRAINT fk_orders_cargo_owner
  FOREIGN KEY (cargo_owner_id) REFERENCES companies(id);
ALTER TABLE orders ADD CONSTRAINT fk_orders_forwarder
  FOREIGN KEY (forwarder_id) REFERENCES companies(id);
ALTER TABLE orders ADD CONSTRAINT fk_orders_operator
  FOREIGN KEY (operator_id) REFERENCES companies(id);
```

## canonical/orders/pseudo-code/confirm.md

```
# Algorithm: Confirm an order (UC-003)

## Preconditions
- Order with id exists. (INV-001)
- Caller is authenticated. (INV-002)
- Caller is a participant: caller.company_id ∈ {order.cargo_owner_id, order.forwarder_id, order.operator_id}. (INV-003)
- Order is in a status where confirmation is meaningful. (INV-005)

## Flow
1. Load order by id.
2. IF order is null → fail with ORDER_NOT_FOUND.
3. IF caller.company_id ∉ {order.cargo_owner_id, order.forwarder_id, order.operator_id}
   → fail with NOT_FOUND (indistinguishable by design — INV-003).
4. Load related sales_order.
5. Branch by caller.company.type_company:
   a. forwarder branch:
      - Set order.status = 'forwarder_confirmed'.
      - Parse pickup_date from order.form.
      - Initialize cargo with available_at = pickup_date. (INV-012)
      - Enqueue cargo markings generation job {attempts=50, backoff=10s}.
      - Emit OrderConfirmedByForwarder event.
   b. cargo_owner branch:
      - Set order.status = 'cargo_owner_confirmed'.
      - Emit OrderConfirmedByCargoOwner event.
6. Update Elasticsearch order index.
7. Return updated order with sales_order attached.

## Side effects
- Database: update status; forwarder path also inserts into cargo.
- Queue: enqueue markings job (forwarder path only).
- Search index: always updated.
- Event bus: OrderConfirmedByForwarder or OrderConfirmedByCargoOwner.

## Invariants preserved
- INV-003 (authorization).
- INV-005 (status transition rules).
- INV-012 (cargo init on forwarder confirm).

## Traceability
- Use case: UC-003.
- Scenario: SC-042.
- Terms: forwarder_confirmed (TERM-012), cargo_owner_confirmed (TERM-013).
```

## canonical/orders/api.graphql

```graphql
# Standalone GraphQL SDL for the Orders domain
# Parses with graphql-js (validated by C11).

enum Orders_StatusEnum {
  created
  accepted
  cargo_owner_confirmed
  forwarder_confirmed
  fulfillment
  paid
  canceled
  completed
}

type Orders_Order {
  id: Int!
  code: String!
  status: Orders_StatusEnum!
  cargo_owner_id: Int!
  forwarder_id: Int!
  operator_id: Int
  quote_id: Int!
  paid: Boolean!
  created_at: String!
  updated_at: String!
}

type Mutation {
  orders_Confirm(id: Int!): Orders_Order!
  orders_Cancel(id: Int!): Orders_Order!
}
```

## canonical/orders/scenarios.feature

```gherkin
Feature: Order confirmation by forwarder
  As a forwarder employee
  I want to confirm an order I have committed to
  So that cargo initialization proceeds and the order can advance toward fulfillment

  Background:
    Given an order O1 with status 'accepted'
    And user U1 is an employee of O1.forwarder
    And user U2 is an employee of a non-participant company
    And user U3 is an employee of O1.cargo_owner

  Scenario: Happy path — forwarder confirms
    When U1 calls orders_Confirm(O1.id)
    Then O1.status becomes 'forwarder_confirmed'
    And cargo is initialized
    And a markings job is enqueued

  Scenario: Non-participant rejected
    When U2 calls orders_Confirm(O1.id)
    Then error '__NOT_FOUND__' is raised

  Scenario: Cargo_owner mirror path
    When U3 calls orders_Confirm(O1.id)
    Then O1.status becomes 'cargo_owner_confirmed'
    And no cargo is initialized
```

## canonical/orders/invariants.md

```markdown
## INV-003 — Order visibility limited to participants
An order is visible only to employees of its `cargo_owner_id`, `forwarder_id`, or `operator_id` company.
Rationale: privacy between business parties; order content is competitive info.
Violation: `__NOT_FOUND__` (indistinguishable from non-existing order).

## INV-005 — Order status transitions
Order status follows a state machine. See state diagram in use-cases.md#state-machine.

## INV-012 — Cargo initialized on forwarder confirm
Cargo is initialized only when the forwarder confirms (not cargo_owner). This is the point at which the forwarder commits transport resources.
```

## canonical/orders/glossary.md (excerpt)

```markdown
## Order
A contract between cargo_owner, forwarder, and optional operator to move goods.

## forwarder_confirmed
Order status indicating the forwarder has committed resources for fulfillment.

## cargo_owner_confirmed
Order status indicating the cargo_owner has confirmed cargo data.

## fulfillment
Order status where transport execution is active; only reachable after both forwarder_confirmed and cargo_owner_confirmed.
```

## Validation evidence

```
C11 reproducibility-validator results:
- ddl_compile: PASS (psql check)
- sdl_parse: PASS (graphql-js)
- gherkin_parse: PASS (@cucumber/gherkin)
- pseudo_code_coherence: PASS (all entities, invariants, terms exist)
- deletion_simulation_reproduction_rate: 82%
```

Note: all rendered sections above use **stable artifact IDs** (UC-003, INV-003, TERM-012) — not `file:line` references. This is the defining property of canonical output.
