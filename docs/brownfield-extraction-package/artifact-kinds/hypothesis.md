# Artifact kind: `hypothesis`

> A first-class, lifecycle-tracked proposition about the meaning of code. Has its own state machine from `drafted` → `verified` / `refuted` / `parked`.

## Purpose

Hypotheses separate **claims being tested** from **established facts**. In brownfield reverse-engineering, every intent-statement starts as a hypothesis. Without this separation, speculation leaks into documentation and gets mistaken for truth.

Hypotheses are first-class artifacts so they can be:
- Queried (`all hypotheses about Orders domain, state=drafted`).
- Triangulated independently.
- Linked to evidence.
- Parked for Domain Owner interviews.
- Promoted to verified knowledge when confirmed.
- Rejected when refuted (and their parked doc paths decayed).

## When to create

- Whenever C3 (intent-inferrer) generates a candidate explanation.
- When code patterns suggest multiple possible meanings.
- When contradictions arise between sources (code says X, comment says Y).
- When Domain Owner gives an answer that needs cross-checking against code.

## Frontmatter schema

```yaml
kind: hypothesis
id: HYP-{auto}
subject: "Why forwarder_confirmed differs from cargo_owner_confirmed"
observation: "Order.status ENUM includes both values; _confirm action branches by caller company type"
bounded_context: "orders"
lifecycle_state: "drafted" | "triangulated" | "verified" | "strong-inferred" | "inferred" | "refuted" | "parked" | "superseded"
candidates:
  - id: "H1"
    statement: "Dual-confirm models a two-party commitment: forwarder commits resources, cargo_owner commits data."
    prior_probability: 0.5
    supporting_evidence: ["EVID-018"]
    refuting_evidence: []
    plausibility_score: 0.7
  - id: "H2"
    statement: "This is a legacy artifact from when only one of them was required."
    prior_probability: 0.2
    supporting_evidence: []
    refuting_evidence: ["EVID-020 — git log shows both added together"]
    plausibility_score: 0.2
  - id: "H3"
    statement: "An authorization workflow for regulatory compliance."
    prior_probability: 0.2
    supporting_evidence: []
    refuting_evidence: []
    plausibility_score: 0.3
selected_candidate: "H1"
confidence_rationale: "Git shows both enum values added in the same commit with _confirm; supports H1 over H2. H3 unsupported."
triangulation_sources:
  - type: "git_history"
    finding: "both enum values added in commit abc123, 2024-01-15, with _confirm"
  - type: "code_comment"
    finding: "comment on line 3420 says 'экспедитор фиксирует ресурсы'"
  - type: "legacy_docs"
    finding: "not mentioned"
  - type: "naming_pattern"
    finding: "consistent *_confirmed pattern across related fields"
code_refs:              # Tier 1 only
  - "models/Order.js:82-91"
  - "services/v5.orders.service.js:_confirm"
related_artifacts:
  - "UC-003"
  - "INV-005"
  - "TERM-012"
  - "TERM-013"
parked_in: "IP-2026-04-21"     # interview packet reference
verification:
  last_reviewed: "2026-04-21"
  days_since_promotion: 0
blocks_artifacts: ["UC-003", "SC-042"]    # artifacts that depend on this hypothesis being verified
```

## Lifecycle

```
drafted → triangulated → verified | strong-inferred | inferred | refuted | parked
                                                                    ↓
                                                              interview answer
                                                                    ↓
                                                            verified | refuted
```

| State | Meaning |
|---|---|
| `drafted` | Proposed but not yet evaluated. |
| `triangulated` | Evaluated against git/docs/comments/naming, assigned a confidence. |
| `verified` | Confirmed by Domain Owner interview. |
| `strong-inferred` | Multiple independent sources agree. |
| `inferred` | Some evidence but incomplete. |
| `refuted` | Evidence contradicts. |
| `parked` | Cannot be resolved from code; awaiting Domain Owner. |
| `superseded` | Replaced by a better-scoped hypothesis. |

## Body structure

1. **Observation** (what was noticed in the code).
2. **Subject** (the question being asked about that observation).
3. **Candidate explanations** (3+ with supporting/refuting evidence).
4. **Selected candidate** (current best guess, with confidence).
5. **Triangulation** (what each source says).
6. **Blocked artifacts** (what depends on resolution).
7. **Next action** (triangulate further / park for interview / accept / refute).

## Example (TripSales)

See the `frontmatter` example above. Full body follows the structure.

```markdown
# HYP-042 — Why dual-confirm pattern for orders?

## Observation
`Order.status` ENUM includes both `forwarder_confirmed` and `cargo_owner_confirmed`. The `_confirm` action branches based on caller company type to set one or the other.

## Subject
Why two separate confirmation states instead of one?

## Candidates

### H1 — Two-party commitment (plausibility 0.7)
Dual-confirm models that forwarder commits resources (trucks, routes) while cargo_owner commits data (cargo details, pickup window). Both commitments must exist before fulfillment starts. This matches standard logistics business where each party has distinct responsibilities.

**Supporting**: code comment line 3420 says "экспедитор фиксирует ресурсы"; fulfillment transition requires both.
**Refuting**: none found.

### H2 — Legacy artifact (plausibility 0.2)
Historical single-confirm pattern that was split in two, without the old value being removed.

**Supporting**: none.
**Refuting**: git log shows both added together in commit abc123.

### H3 — Regulatory compliance workflow (plausibility 0.3)
Dual confirm used to satisfy regulatory requirement for transparent two-party agreement.

**Supporting**: none.
**Refuting**: no regulatory documents found that describe this.

## Selected
H1, confidence `inferred` (one strong signal from comment + code structure, but no Domain Owner confirmation).

## Next action
Parked in interview packet IP-2026-04-21, question Q3. When DO answers, update to `verified` or `refuted`.

## Blocked artifacts
- UC-003 (confirm use-case description depends on this).
- SC-042 (happy path scenario).
- TERM-012, TERM-013 (glossary definitions).
```

## Validation rules

- At least 3 candidates (except for `verified` or `refuted` states which collapse to 1).
- `selected_candidate` must exist in `candidates`.
- `lifecycle_state` transitions must follow the state machine.
- Refuted hypotheses keep their body for historical reference; do not delete.

## Links

- `infers_from` — (hypothesis) infers_from (observation).
- `parked_in` — (hypothesis) parked_in (interview-packet / note).
- `resolved_by` — (hypothesis) resolved_by (interview-answer) when DO confirms.
- `blocks` — (hypothesis) blocks (use-case | invariant | scenario).
- `contradicts` — (hypothesis) contradicts (hypothesis | invariant).

## Freshness

Parked hypotheses age. If `parked` > 30 days, escalate priority in next interview packet.
