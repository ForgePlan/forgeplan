# Skill: canonical-reproducer (C10)

> Produces self-contained standalone documents (DDL, pseudo-code, SDL) with zero `file:line` references in final sections.

## Why this skill exists

Docs that reference `services/v5.orders.service.js:3431` break when code is moved or deleted. A business documentation package must survive code deletion. C10 converts verified knowledge into standalone canonical form.

## Input

- All verified artifacts:
  - `domain-model` with `canonical_ddl` and `canonical_pseudo_code`.
  - `use-case` fully traced.
  - `invariant` with statement.
  - `scenario` Gherkin valid.
  - `glossary` with definition.
- Schema evolution history (from factum SPECs).
- Dataset of all GraphQL types (factum).

## Output

Per-domain canonical documents:
- `canonical/{domain}/README.md` — domain overview.
- `canonical/{domain}/ddl.sql` — standalone schema.
- `canonical/{domain}/api.graphql` — standalone SDL.
- `canonical/{domain}/pseudo-code/{action}.md` — per-action pseudo-code.
- `canonical/{domain}/scenarios.feature` — combined Gherkin.
- `canonical/{domain}/glossary.md` — domain glossary subset.
- `canonical/{domain}/invariants.md` — domain invariants.
- `canonical/{domain}/use-cases.md` — domain use-cases.
- `canonical/{domain}/sequence-diagrams.md` — Mermaid sequences.

**Rule**: final rendered sections MUST NOT contain `file:line` references. They may contain references to OTHER canonical sections by stable ID.

## Modes

### Mode 1: `render-all`
Render every domain.

### Mode 2: `render-domain`
Render single domain.

### Mode 3: `diff`
Compare rendered docs against source artifacts (detect drift).

## Algorithm

### Render

For each domain:
1. Collect all linked artifacts (via KG from C9).
2. Filter: keep only confidence ≥ inferred (exclude speculation).
3. Sort by dependency (glossary first, then invariants, then use-cases, then scenarios).
4. Render per template (see `templates/`).
5. Substitute all `<code_ref>` placeholders with canonical text (extracted from code + promoted).

### DDL rendering

From `domain-model.canonical_ddl` field, aggregate + sort dependencies:
```sql
-- canonical/orders/ddl.sql

-- Enum types first
CREATE TYPE order_status AS ENUM (
  'created', 'accepted', 'cargo_owner_confirmed',
  'forwarder_confirmed', 'fulfillment', 'rejected',
  'canceled', 'completed', 'removed'
);

-- Tables with deferred FKs
CREATE TABLE orders (
  id SERIAL PRIMARY KEY,
  code VARCHAR NOT NULL UNIQUE,
  quote_id INT NOT NULL,
  -- ... full column list from verified schema
);
-- ...
ALTER TABLE orders ADD CONSTRAINT fk_orders_quote FOREIGN KEY (quote_id) REFERENCES quotes(id);
```

### Pseudo-code rendering

From `use-case.steps` + `invariants_invoked`:
```
# canonical/orders/pseudo-code/confirm.md

## Algorithm: Confirm an order

### Preconditions
- order must exist (invariant INV-001: non-null ID).
- caller must be a participant (invariant INV-003: authorization).

### Flow

1. Load order by id.
2. IF order is null → fail with ORDER_NOT_FOUND.
3. IF caller.company_id ∉ {order.cargo_owner_id, order.forwarder_id, order.operator_id} → fail with UNAUTHORIZED.
4. Load the related sales_order.
5. IF caller is a forwarder employee:
   a. Set order.status to FORWARDER_CONFIRMED.
   b. Parse pickup_date from the order's form.
   c. Initialize cargo_available_at based on pickup_date.
   d. Enqueue cargo markings generation (retry up to 50x, exponential backoff).
6. OTHERWISE (cargo_owner employee):
   a. Set order.status to CARGO_OWNER_CONFIRMED.
7. Update the search index.
8. Return the updated order with sales_order attached.

### Side effects
- Database: update status, init cargo.
- Queue: enqueue markings job.
- Search index: update.

### Invariants preserved
- INV-003 (authorization).
- INV-005 (status transition rules).
```

### GraphQL SDL rendering

From all `graphql_type` extracts, deduplicate and present:
```graphql
# canonical/orders/api.graphql

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
  # ...
}

type Mutation {
  orders_Confirm(id: Int!): Orders_Order!
  # ...
}
```

### Cross-reference rule

Final sections use stable IDs not file:lines:
- Refer to glossary terms by `[term](../glossary.md#term-anchor)`.
- Refer to invariants by `[INV-003](../invariants.md#inv-003)`.
- Refer to use-cases by `[UC-007](../use-cases.md#uc-007)`.

## Metric

- `zero_file_line_refs`: must be 100% in final rendered sections.
- `domains_rendered / total_domains`: target ≥ 0.95.
- `validation_pass_rate`: C11 validator must pass on ≥ 90% of rendered docs.

## Dependencies

- C1-C9 outputs.
- Template engine (simple text templates suffice).

## Integration with autoresearch

`/autoresearch:learn --mode canonical`:
- Uses learn's validate-fix loop (C11 is the validator).

## Prompt templates

See `references/render-ddl.md`, `references/render-pseudo-code.md`, `references/render-sdl.md`.

## Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| DDL generates invalid SQL | Fail on `psql --check` | Regenerate with fix-up prompt |
| Pseudo-code contradicts scenarios | C11 validator flags | Route to C6 re-triangulation |
| Missing canonical artifact | `forgeplan_blindspots` | Escalate to C7 interview |
| Stale output | Factum changed after render | Re-render on change notification |

## Example

See `examples/tripsales-canonical-orders-sample.md` for a partial render.

## Testing

Fixture: one fully-verified domain → expect complete canonical output, all references resolved, DDL lints.

## Version history

- v1.0.0 — initial design.
