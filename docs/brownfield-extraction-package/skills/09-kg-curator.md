# Skill: kg-curator (C9)

> Builds and maintains the knowledge graph connecting glossary terms, use-cases, invariants, scenarios, hypotheses, domain-models. Detects contradictions.

## Why this skill exists

All other skills produce artifacts. Without linking and contradiction detection, the workspace is a bag of documents. The KG is the **overall coherence layer**.

Specific gaps it fills:
- Contradiction detection: two invariants that conflict.
- Coverage gap detection: a use-case with no scenarios.
- Orphan detection: an artifact not linked to anything.
- Tier-based navigation: drill down from overview to details.
- Semantic search: answer business questions by traversing graph.

## Input

- All artifacts in the forgeplan workspace.
- All relations (existing + new from this package).

## Output

- Graph structure stored in forgeplan (leverages `forgeplan_graph` but extended).
- **Contradiction reports**.
- **Coverage gap reports**.
- **Orphan reports**.
- **Overview navigation** markdown — tier-0 (domains) → tier-1 (entities) → tier-2 (artifacts).

## Modes

### Mode 1: `build`
Fresh graph from current workspace.

### Mode 2: `incremental`
Update graph on artifact change.

### Mode 3: `check-contradictions`
Find conflicting artifacts.

### Mode 4: `check-coverage`
Find gaps (use-case without scenarios, invariant without verification, etc.).

### Mode 5: `navigate`
Generate navigation indices.

## Algorithm

### Build

Nodes:
- Artifacts (by id).
- Glossary terms (node type `term`).
- Code symbols referenced by artifacts (node type `code-symbol`, tier-2).
- Events (node type `event`, from causal links).

Edges (from artifact relations + inferred):
- `refines`, `informs`, `based_on`, `supersedes`, `contradicts` — existing forgeplan.
- `defines`, `triggers`, `verifies`, `infers_from`, `resolved_by`, `parked_in`, `catalogs`, `emitted_by`, `causes`, `mutates`, `listens_to`, `loop` — new.

### Check contradictions

Candidate contradictions:
1. Two invariants A and B where A's statement is the negation of B's statement (LLM check).
2. Two hypotheses about the same observation, both marked `verified`.
3. Use-case outcome contradicts invariant (outcome states X, invariant says NOT X).
4. Scenario step contradicts invariant.
5. Glossary term definitions that diverge (same term, different domain, but claimed synonymous).
6. Causal cycle from C5 already flagged.

For each, create a `problem` artifact with `kind: contradiction` and link both conflicting artifacts with `contradicts`.

### Check coverage

Rules:
- Use-case must have ≥ 1 scenario (if not, flag as `uncovered_use_case`).
- Invariant should be `verifies`'d by ≥ 1 scenario (if not, flag as `unverified_invariant`).
- Glossary term must appear in ≥ 1 use-case/invariant/scenario (if not, flag as `orphan_term`).
- Hypothesis must have ≥ 1 triangulation source (if not, flag as `un-triangulated`).
- Domain-model must have `canonical_ddl` (if not, flag as `incomplete_model`).

### Navigate

Build tier views:
- Tier-0 (domain overview): per bounded context, list use-cases + aggregate roots.
- Tier-1 (entity overview): per entity, list actions, states, invariants.
- Tier-2 (artifact details): direct artifact content.

Navigation markdown:
```markdown
# Knowledge Graph — Tier 0 (Domain Overview)

## Orders
- Aggregate roots: [Order](...), [Shoulder](...)
- Use cases (12): [Create](...), [Confirm](...), [Cancel](...), ...
- Invariants (18): [INV-003](...), ...
- Open hypotheses (7): [HYP-012](...), ...

## Trips
...

## Sales
...
```

## Metric

- `graph_density`: edges / max possible edges.
- `orphans_count`: target ≤ 5%.
- `contradictions_count`: any is a problem.
- `coverage_rate`: % artifacts with full linkage.

## Dependencies

- All other skills' artifacts.
- Forgeplan `forgeplan_graph` tool.
- Optional: graph database for larger workspaces.

## Integration with autoresearch

No direct integration — curator is a continuous process, not an iteration.

## Prompt templates

See `references/contradiction-detection.md` and `references/coverage-check.md`.

## Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| False contradiction (LLM too aggressive) | Human review | Require confidence threshold before flagging |
| Graph too dense to visualize | > 10k edges | Tier-based collapse; sample views |
| Circular dependencies in tier hierarchy | Detection during navigation build | Treat cycles as separate tier-level cluster |

## Example

Contradiction example for TripSales:
- INV-005: "Orders transition to fulfillment only after both forwarder and cargo_owner confirm."
- Observed behavior (from factum): transition happens on trip start, which can happen after only forwarder confirms.
- Flag: INV-005 contradicts observed-factum. Resolution: either INV-005 is wrong (single confirmation is enough), OR observed-factum is a bug.

## Testing

Fixture: workspace with known contradictions + orphans → expect all flagged.

## Version history

- v1.0.0 — initial design.
