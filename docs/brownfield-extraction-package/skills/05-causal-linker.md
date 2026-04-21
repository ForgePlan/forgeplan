# Skill: causal-linker (C5)

> Maps actions, events, and side-effects into causal chains. Builds "X causes Y" relations across the codebase.

## Why this skill exists

Business behavior is not per-function; it's end-to-end. "Invoice gets issued" is not just `_issueInvoice`; it also publishes an event, updates items, adjusts sales order, and affects downstream `cancel.payment` semantics. Scenarios and domain-models need this causal graph.

## Input

- **Scope**: all service files.
- **Use-cases** (C2 output) for entry-point bindings.
- **Events catalog** (from existing documentation, e.g., SPEC-013).

## Output

Graph edges in forgeplan (using new relation types):
- `causes`: action A → outcome B (direct call, synchronous).
- `emits`: action → event (async).
- `listens_to`: handler ← event.
- `loop`: cycle marker (A → B → ... → A).
- `mutates`: action → entity state (DB write or in-memory change).
- `requires`: action → precondition invariant.

Plus: metadata artifacts for bulk causal chains (per entity or per use-case).

## Modes

### Mode 1: `trace-static`
Per service file, enumerate all `ctx.call`, `broker.emit`, `this.createJob`, DB writes, ES indexing.

### Mode 2: `resolve-handlers`
For each emitted event, find listener(s) in `events: {...}` blocks.

### Mode 3: `chain`
Build chains from entry points through N hops.

### Mode 4: `detect-cycles`
Find A → B → ... → A patterns.

### Mode 5: `aggregate`
Aggregate per-entity: for each entity, list all actions that mutate it.

## Algorithm

### Static trace

For each action:
1. Parse handler body AST.
2. Collect:
   - `ctx.call(actionName, ...)` with argument shape.
   - `broker.emit(eventName, ...)` with payload shape.
   - `broker.broadcast(eventName, ...)` — same.
   - `this.createJob(queueName, ...)` — queue push.
   - `throw new Error(code)` — error emission.
   - DB mutations (via ORM or raw SQL): `update`, `create`, `destroy`, `set`.
3. Record each as an edge.

### Resolve handlers

Match emitted event names to listener entries:
- `events: { 'event.name': handler }` blocks.
- Queue consumers (names must match producer names).
- Subscription filters (for GraphQL).

Flag **dangling emits** (no listener) and **dangling listeners** (no emitter) as problems.

### Chain

Depth-first from entry point:
1. Start at use-case entry point.
2. For each step's `ctx.call`, recurse into target action.
3. Track path; limit depth 10 or domain boundary.
4. Emit chain as sequence of edges.

### Cycle detection

Standard graph DFS with visit stack:
- When a revisit to a node already on stack → cycle.
- Record cycle members as a `loop` cluster.
- Separate "self-call" loops (A → A, often legitimate) from longer cycles.

### Aggregate

For each entity (from models/):
1. Find all actions that mutate it (from static trace).
2. Group by use-case (backward via `causes` edges).
3. Generate per-entity summary:
   - Entry points that trigger mutations.
   - Sequence of state transitions.
   - External services invoked.

## Metric

- `resolved_edges / total_edges`: target ≥ 0.95 (unresolved are dangling).
- `cycles_detected`: count; threshold for alert = > 5 (architectural smell).
- `chains_per_use_case`: average.
- `dangling_emit_count`, `dangling_listener_count`: target 0.

## Dependencies

- Forgeplan: new relations.
- C2 (use-cases) for entry points.
- AST parser.

## Integration with autoresearch

`/autoresearch:predict --persona causality-analyst`:
- Multi-persona analysis:
  - **Architect**: "is this causal graph clean or tangled?"
  - **Performance Engineer**: "are there synchronous chains that should be async?"
  - **Reliability Engineer**: "are there failure cascade paths?"
  - **Devil's Advocate**: "could cycles be hiding bugs?"

## Prompt template

See `references/resolve-handlers-prompt.md`.

## Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| Dynamic call (`ctx.call(variable)`) can't resolve | AST shows non-literal | Flag as "dynamic dispatch" + list potential candidates via name pattern |
| Event name mismatch (emitter uses tag, listener uses topic) | Mismatch in grep | Require exact literal string matches; alert on near-matches |
| Library internals masquerade as events | Noise | Filter: only events defined in project code |
| Transitive explosion | > 1000 edges | Tiered view: high-level (service-service) vs low-level (action-action) |

## Example

For TripSales:
- Entry point: `orders_Confirm(id)` mutation.
- Chain: `v5.orders.confirm → _confirm → v5.db.orders.update.status → (emit v5.db.orders.update.status event, DANGLING) → v5.orders.elastic.add → queue v5.orders.generate.cargo.markings → v5.orders.generate.cargo.markings handler`.

## Testing

Fixture: known chains with known cycles → expect correct graph + cycle detection.

## Version history

- v1.0.0 — initial design.
