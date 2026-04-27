# Skill: use-case-miner (C2)

> Maps entry points to user journeys — from trigger to business outcome.

## Why this skill exists

Use cases capture **flows** in the system at a business level: "Forwarder confirms an order" is a use case; `v5.orders.service.js:_confirm` is implementation detail. Scenarios, invariants, and canonical docs all depend on a use-case catalog.

## Input

- **Scope**: service files, gateway config, queue definitions.
- **Entry-point hints** (optional): known ingress patterns (GraphQL mutations, REST routes, queue producers, scheduled jobs, REPL commands).
- **Existing glossary** (from C1): for term consistency.

## Output

`use-case` artifacts.

Frontmatter:
```yaml
kind: use-case
id: UC-{auto}
name: "Forwarder confirms an order"
domain: "orders"
actor: "forwarder employee"
entry_point:
  type: "graphql_mutation" | "rest_endpoint" | "queue_job" | "scheduled" | "repl" | "event"
  ref: "orders_Confirm(id: Int!)"
trigger: "User clicks 'Confirm' button in forwarder UI"
preconditions:
  - "Order exists and has status in {created, cargo_owner_confirmed}"
  - "Authenticated user belongs to company with type_company='forwarder'"
steps:
  - step: 1
    description: "Load order by id"
    tier: factum
    code_ref: {file: "...", line: ...}
  - step: 2
    description: "Authorize: user.company_id ∈ {order.cargo_owner_id, order.forwarder_id, order.operator_id}"
    tier: factum
    invariant_ref: INV-003
  # ...
outcome:
  success: "Order transitions to forwarder_confirmed, cargo markings job enqueued"
  failures:
    - "Unauthorized → __NOT_FOUND__"
    - "Already confirmed by forwarder → no-op (hypothesis)"
invariants_invoked: [INV-005, INV-012]
events_emitted: [OrderStatusChanged, OrderConfirmedByForwarder]
side_effects:
  - db_writes: ["orders.status update", "order_cargo init"]
  - queue_jobs: ["v5.orders.generate.cargo.markings"]
  - indexing: ["ES index update"]
  - external: []
related_use_cases: [UC-008, UC-012]
confidence: inferred
```

Body sections:
1. **Overview** (2-3 sentences, intent-level).
2. **Preconditions** (detailed).
3. **Steps** (numbered, each with factum/intent tagging).
4. **Outcome and failure modes**.
5. **Side effects**.
6. **Related use cases**.
7. **Open questions** for Domain Owner.

## Modes

### Mode 1: `enumerate`
Find all entry points in scope.

```
output: <entry_point_type, ref, definition_location>[]
```

### Mode 2: `trace`
For each entry point, follow execution flow.

```
input: entry point
output: ordered step list with code refs and side effects
```

### Mode 3: `synthesize`
Aggregate traces into use-case artifacts (business-level summaries).

```
input: trace
output: use-case artifact
```

## Algorithm

### Enumerate phase

1. **GraphQL mutations**: grep `mutation: \`` blocks + `query: \`` blocks in service files.
2. **REST endpoints**: inspect gateway service (e.g., `apiGateway.service.js`) for routes.
3. **Queue producers**: grep `createJob(` and `.add(` patterns.
4. **Scheduled**: grep cron or schedule keywords.
5. **REPL**: parse `moleculer.config.js` `customCommands` or equivalent.
6. **Event emitters**: grep `broker.emit`, `broker.broadcast`, `ctx.emit`.
7. **Authenticated action surface**: any `grants:` metadata on actions.

### Trace phase

For each entry point:
1. Follow the action handler → internal method calls → db calls → side effects.
2. Limit trace depth (max 10 levels; stop at DB layer or external call).
3. Record each `ctx.call` / `this.call` with its argument shape.
4. Record each `throw` and its error code.
5. Record each event emit / queue push / ES indexing.
6. Record state changes (`status: 'X'` assignments).

### Synthesize phase

1. LLM prompt:
   ```
   Given this trace of code, write a business-level use case.
   - Actor: {inferred from auth check}
   - Trigger: {entry point signature}
   - Preconditions: {from auth + validation checks}
   - Steps: {summarize the trace at business level, merging consecutive factum steps}
   - Outcome: {state transitions + side effects}
   ```
2. Reference invariants from C4 output (if available).
3. Assign tier+confidence per step.

## Metric

`coverage = mapped_entry_points / total_entry_points` (target ≥ 0.8).

Quality sub-metrics:
- `% with actor identified` (not "unknown").
- `% with preconditions listed`.
- `% with failure modes listed`.
- `% linked to ≥ 1 invariant`.

## Dependencies

- Forgeplan MCP: artifact kind `use-case`, new relations `triggers, causes, emits`.
- C1 output (glossary) for term consistency.
- Optionally C4 output (invariants) for linking.

## Integration with autoresearch

`/autoresearch:learn --mode use-case`:
- Scout for entry points.
- Validate-fix loop: ensure each use-case has actor + trigger + outcome.

`/autoresearch:scenario --source use-case`:
- Generate scenarios from use cases (C8 integration).

## Prompt template

See `references/trace-prompt.md` and `references/synthesize-prompt.md` (to be created).

## Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| Trace too deep (crosses service boundaries unnecessarily) | Trace > 20 steps | Cap depth, mark as "complex use case, split" |
| Actor ambiguous | Multi-actor code paths | Generate multiple use cases, one per actor branch |
| Entry point is internal (not user-facing) | Not called from outside | Skip (internal action, not a use case) |
| Trace misses async side effect | Queue job not followed | Post-hoc linking: if job consumer exists, add as side-effect |

## Examples

See `examples/tripsales-use-case-sample.md`.

## Testing

Fixture: `fixtures/use-case-fixture/` with 3 entry points, expected use-cases documented.

## Version history

- v1.0.0 — initial design.
