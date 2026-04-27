# Skill: invariant-detector (C4)

> Extracts business rules from code guards (`if/throw/assert`) and groups them semantically.

## Why this skill exists

Code guards encode business rules implicitly. Examples:
- `if (!user || user.company_id ∉ {order.*_id}) throw '__NOT_FOUND__'` → **rule**: Orders are visible only to their participants.
- `if (order.status === 'forwarder_confirmed')` → **rule**: Certain actions only apply after forwarder commits.

Without extraction, rules live in code, invisible to auditors, PMs, and anyone reading docs. Bugs often stem from rule violations that go unnoticed.

## Input

- **Scope**: service files, db-layer files.
- **Glossary** (from C1) for term consistency.

## Output

`invariant` artifacts.

Frontmatter:
```yaml
kind: invariant
id: INV-{auto}
statement: "Orders are visible only to cargo_owner, forwarder, or operator"
domain: "orders"
category: authorization | lifecycle | data_integrity | financial | audit
severity: critical | high | medium | low
scope:
  entity: "Order"
  operations: ["read", "list"]
violation_consequence: "Unauthorized users could see orders they shouldn't → data leak, privacy breach"
code_refs:
  - file: "services/v5.orders.service.js"
    lines: [954-961]
    context: "get.one authorization check"
  - file: "services/v5.orders.service.js"
    lines: [3410-3418]
    context: "_confirm authorization check"
verification:
  test_hint: "Integration test: user from company X tries to fetch order of company Y → expect __NOT_FOUND__"
related_invariants: [INV-007]
related_use_cases: [UC-003, UC-005, UC-008]
confidence: inferred | verified
created_by: skill:invariant-detector v1.0.0
```

Body sections:
1. **Statement** (declarative, human-readable).
2. **Rationale** (intent, from C3 if available).
3. **Enforcement points** (list of code refs).
4. **Verification** (how to test).
5. **Known exceptions** (e.g., REPL bypass via `$repl`).
6. **Related invariants**.

## Modes

### Mode 1: `scan`
Find all guard patterns.

```
output: <location, condition, throw_error>[]
```

### Mode 2: `classify`
Categorize each guard: authorization / lifecycle / data_integrity / financial / audit.

### Mode 3: `group`
Group similar guards across files into logical invariants.

### Mode 4: `verify-gaps`
Find places where an invariant should be enforced but isn't (e.g., one action has the check, sibling doesn't).

## Algorithm

### Scan phase

Patterns:
- `if (condition) throw ...`
- `if (!condition) throw ...`
- `assert(...)`.
- Moleculer `params:` schema guards (declarative validation).
- Sequelize `validate:` blocks.
- GraphQL `@deprecated` directives carrying intent.

Collect:
- Location (`file:line`).
- Condition expression (AST form).
- Error code / message.
- Surrounding context (action name).

### Classify phase

Heuristics:
- Condition mentions `user`, `company_id`, `grants`, `role` → **authorization**.
- Condition mentions status / state transition → **lifecycle**.
- Condition mentions `null`, `undefined`, type checks → **data_integrity**.
- Condition mentions `paid`, `invoice`, `amount`, `balance` → **financial**.
- Error code includes `AUDIT`, `HISTORY`, `LOG` → **audit**.

Fallback: LLM classification with condition + context.

### Group phase

Cluster guards by:
- Same error code (often indicates same rule).
- Same entity + similar condition pattern.
- Structural similarity (Levenshtein / AST similarity).

For each cluster, LLM generates a canonical invariant statement.

### Verify-gaps phase

For each invariant, find places where it should apply (same entity + same operation category) but the guard is missing. Flag as:
```yaml
invariant: INV-003 (Orders visible only to participants)
applied_in:
  - services/v5.orders.service.js:954 (get.one)
  - services/v5.orders.service.js:3410 (_confirm)
MISSING in:
  - services/v5.orders.service.js:1953 (getFilteredIds) # see PROB-014.4
  - services/v5.orders.service.js:1937 (get.shoulder) # see PROB-014.1
```

Creates problem artifacts for each gap.

## Metric

- `detected_guards / total_guards_in_scope`: target ≥ 0.8.
- `classified_guards / detected_guards`: target ≥ 0.95.
- `invariants_per_category`: should be non-zero in each category.
- `gap_detection_rate`: true gaps flagged / total gaps (validated against ground truth).

## Dependencies

- Forgeplan: `invariant` kind.
- C1 (glossary) for entity names.
- AST parser for the target language (Babel for JS).

## Integration with autoresearch

`/autoresearch:learn --mode invariant`:
- Scout for guard patterns.
- Validate-fix: ensure each invariant has statement + severity + violation_consequence.

## Prompt template

See `references/classify-prompt.md` and `references/group-prompt.md`.

## Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| Guard not a business rule (e.g., null-check for safety) | Low-severity classification | Filter at severity threshold; still catalog for completeness |
| Over-grouping (merging distinct rules) | LLM review | Require human review for cross-file clusters |
| Under-grouping (treating same rule as different) | Many similar single-item invariants | Re-run cluster with looser threshold |
| Verify-gaps false positives | Context-dependent differences | LLM check before flagging as problem |

## Example

Given code:
```js
if (!user || user.company_id !== order.cargo_owner_id
    && user.company_id !== order.forwarder_id
    && user.company_id !== order.operator_id) {
  throw new Error('__NOT_FOUND__');
}
```
(repeated in 4 actions)

Expected output: one `invariant` with statement *"An Order is visible only to its cargo_owner, forwarder, or operator"*, linked to 4 code locations, category `authorization`, severity `critical` (security implication).

## Testing

Fixture: 5 files with 20 known guards → expect ≥ 16 detected, correctly classified into 4-6 invariants.

## Version history

- v1.0.0 — initial design.
