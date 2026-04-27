# Skill: scenario-writer (C8)

> Converts verified use cases + invariants into Gherkin Given/When/Then scenarios with Mermaid sequence diagrams.

## Why this skill exists

Scenarios are the **executable form** of business documentation. They can become:
- Test specs (for test generation).
- Onboarding material (step-by-step stories).
- RAG input (concrete examples).
- Rewrite specifications.

## Input

- `use-case` artifacts (verified or inferred).
- `invariant` artifacts (linked).
- Existing scenarios (for delta update).

## Output

`scenario` artifacts.

Frontmatter:
```yaml
kind: scenario
id: SCENARIO-{auto}
feature: "Order confirmation by forwarder"
use_case_ref: UC-003
invariants_verified: [INV-005, INV-012]
gherkin_feature: |
  Feature: Order confirmation by forwarder
    As a forwarder employee
    I want to confirm an order has been committed
    So that fulfillment can begin

    Background:
      Given an order O1 with status 'created'
      And quote Q1 was accepted to create O1
      And user U1 is an employee of company C_forwarder
      And O1.forwarder_id = C_forwarder.id

    Scenario: Forwarder confirms in the happy path
      When U1 calls orders_Confirm(O1.id)
      Then O1.status becomes 'forwarder_confirmed'
      And cargo initialization is triggered with available_at from form
      And cargo markings job is enqueued (attempts=50, backoff=10s)
      And Elasticsearch index is updated
      And a resultOrder is returned with sales_order attached

    Scenario: Non-participant is rejected
      Given user U2 is an employee of company C_other
      And C_other.id not in {O1.cargo_owner_id, O1.forwarder_id, O1.operator_id}
      When U2 calls orders_Confirm(O1.id)
      Then an error __NOT_FOUND__ is raised
      And O1.status is unchanged

    Scenario: Cargo_owner path (mirror)
      Given user U3 is an employee of company C_cargo_owner
      And C_cargo_owner.id = O1.cargo_owner_id
      When U3 calls orders_Confirm(O1.id)
      Then O1.status becomes 'cargo_owner_confirmed'
      And no cargo markings job is enqueued
      And Elasticsearch is updated
visualizations:
  - type: mermaid-sequence
    content: |
      sequenceDiagram
          participant FE
          participant GW as Gateway
          participant O5 as v5.orders
          participant DB as v5.db.orders
          participant DBSO as v4.db.sales-orders
          participant Q as Bull Queue
          participant ES as ES index

          FE->>GW: mutation orders_Confirm(O1.id)
          GW->>O5: v5.orders.confirm
          O5->>DB: get.by.id
          Note over O5: auth check
          O5->>DBSO: get.by.order_id
          O5->>DB: update.status('forwarder_confirmed')
          O5->>DB: init.cargo(...)
          O5->>Q: createJob cargo.markings
          O5->>ES: elastic.add
          O5-->>GW: resultOrder
          GW-->>FE: Orders_Order
verification:
  automated: "Integration test: see tests/orders/confirm-by-forwarder.test.js (to be written)"
  manual: null
confidence: inferred | verified
```

Body sections:
1. **Feature overview** (2 sentences).
2. **Background** (common setup).
3. **Happy path scenario**.
4. **Failure scenarios** (authorization, precondition violation, transient failures).
5. **Visualizations** (Mermaid).
6. **Verification** (automated / manual).
7. **Traceability** (references to use-case, invariants).

## Modes

### Mode 1: `generate`
For a use case, produce scenarios.

### Mode 2: `validate`
Check Gherkin syntax.

### Mode 3: `update`
When use-case / invariant changes, re-generate scenarios.

## Algorithm

### Generate

Prompt template:
```
System:
You convert business use cases into Gherkin scenarios. Scenarios must be
executable specifications — concrete enough to derive tests from.

User:
Use case: {use_case}
Invariants: {invariants}

Generate:
1. Feature block (name + brief description).
2. Background (common Given steps).
3. Happy path scenario.
4. 2-3 failure scenarios (authorization, precondition violation, data error).
5. Mermaid sequence diagram of the happy path.

Rules:
- Use Gherkin syntax strictly (Given / When / Then).
- Background should only contain truly common setup.
- Each failure scenario focuses on ONE failure mode.
- Mermaid must include all actors from the use case.
```

### Validate

Use a Gherkin parser (e.g., `@cucumber/gherkin`) to verify syntax. If errors, LLM re-generation with error feedback.

### Update

On use-case change:
1. Diff old vs new use-case.
2. For each affected scenario, mark `needs-update`.
3. Re-generate or manual edit.

## Metric

- `scenarios_per_use_case`: target 3-5 (happy + 2-4 failure).
- `gherkin_valid_rate`: target 100%.
- `scenarios_with_mermaid`: target ≥ 80%.

## Dependencies

- C2 (use-cases).
- C4 (invariants).
- Gherkin parser for validation.

## Integration with autoresearch

`/autoresearch:scenario --template gherkin`:
- Autoresearch's scenario skill already explores edge cases.
- Our contribution: Gherkin template + Mermaid + traceability.

## Prompt template

See `references/generate-scenario.md` and `references/mermaid-template.md`.

## Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| Gherkin invalid | Parser error | Retry with syntax hints |
| Missing failure scenario | < 2 scenarios per feature | Add prompt instruction "always include at least one authorization failure" |
| Mermaid broken | Render error | Validate Mermaid syntax before save |
| Traceability lost | No invariant refs | Require at least one invariant reference per scenario |

## Example

See `examples/tripsales-scenario-sample.md`.

## Testing

Fixture: 5 verified use-cases → expect 15-25 scenarios, all Gherkin-valid.

## Version history

- v1.0.0 — initial design.
