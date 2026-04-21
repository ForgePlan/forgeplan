# Skill: intent-inferrer (C3)

> Given a code pattern, generates 3+ candidate hypotheses explaining **why** it's this way.

## Why this skill exists

Code shows **what** the system does. Business documentation requires **why**. Intent is not directly in code — it's inferred. Multiple explanations are often plausible; without explicit hypothesis generation, agents tend to pick the first plausible explanation and present it as fact.

## Input

- **Observation**: code pattern (snippet or cross-file pattern) from factum layer.
- **Context**: glossary (C1), use-case containing it (C2), invariants involved (C4), domain-specific hints.
- **Hypothesis budget**: how many alternatives to generate (default: 3-5).

## Output

`hypothesis` artifacts (one per alternative, all linked to the same observation).

Frontmatter:
```yaml
kind: hypothesis
id: HYP-{auto}
subject: "Why _cancel only cascades for forwarder_confirmed"
observation: |
  In services/v5.orders.service.js:3498, _cancel only cancels shoulders/points/cargo
  and notifies v4.sales when the prior status was 'forwarder_confirmed'.
hypothesis: "Physical resources (cargo bookings, carrier slots) only get allocated
  at forwarder_confirmed; earlier statuses have nothing to release."
alternatives:
  - id: HYP-{auto}-alt-1
    text: "Other statuses use a different cancellation path in v4.sales."
  - id: HYP-{auto}-alt-2
    text: "Historical bug — cascade should extend to later statuses too."
  - id: HYP-{auto}-alt-3
    text: "Business decision: cancellation after fulfillment requires manual intervention."
predicted_evidence:
  - test: "grep 'update.status.*canceled' in v4.sales"
    predicts: "alt-1 supported if match exists"
  - test: "git log -p for when cascade was added"
    predicts: "H1 supported if added with initial feature; alt-2 supported if added incrementally"
  - test: "UI gateway behavior on cancel for fulfillment/completed orders"
    predicts: "alt-3 supported if UI blocks"
triangulation_sources: [git, legacy_docs, code_comments]
confidence: drafted
state: active
created_by: skill:intent-inferrer v1.0.0
```

## Modes

### Mode 1: `pattern-detect`
Scan factum artifacts for patterns worth inferring:
- Branching by company type / role.
- State-machine quirks (dead enum values, partial cascades).
- Magic numbers or hardcoded IDs.
- Commented-out code (why was it disabled?).
- Naming inconsistencies.
- Compensating workarounds (e.g., `rescue`, `fix` REPL commands).

### Mode 2: `infer`
For each pattern, generate hypotheses via ADI cycle.

### Mode 3: `link`
Link generated hypotheses to their subject artifacts (use-case, invariant, glossary term).

## Algorithm

### Pattern detection

Heuristics:
- `if (user.company.type_company === 'X')` — business-role branching.
- Commented-out code blocks > 5 lines — intentional-disable.
- Magic number in financial / security context.
- Multiple "status" enums across files for same entity — drift.
- `TODO` / `FIXME` / `HACK` comments.
- Inconsistent error codes (e.g., 5 variants of ACCESS_DENIED).
- Closed set of conditions missing one (e.g., enum has 9 values, code handles 7).
- Commits that add a single `if` to an existing method.

### Inference (ADI cycle)

Use multi-persona judge system (inspired by autoresearch `:reason`):

**Personas**:
1. **Business Analyst** — "what business reason would motivate this?"
2. **Technical Historian** — "could this be legacy / migration artifact?"
3. **Domain Lawyer** — "could this be regulatory / compliance?"
4. **Anti-Pattern Detective** — "could this be a bug or accidental design?"
5. **Devil's Advocate** — mandatory dissent, finds weakness in majority view.

Prompt template:
```
System:
You are multiple expert personas analyzing a code pattern's business intent.
Each persona proposes ONE hypothesis. You are the Devil's Advocate. You must
disagree with the consensus.

User:
Observation: {observation}
Context:
  - Glossary: {relevant terms}
  - Use case: {if any}
  - Invariants: {if any}

Task:
1. Each of {5} personas generates ONE hypothesis + rationale.
2. All hypotheses must be semantically DIFFERENT (not paraphrases).
3. For each: list 1-3 testable predictions (deduction).
4. Rank by plausibility.

Output JSON:
{
  "hypotheses": [
    {
      "id": "H1",
      "persona": "business_analyst",
      "hypothesis": "...",
      "rationale": "...",
      "predictions": ["..."],
      "plausibility": 0.0-1.0
    },
    // ... 5 hypotheses
  ],
  "devils_advocate_note": "...",
  "consensus": "H1" | "no consensus"
}
```

### Linking

- Each hypothesis `infers_from` the observation.
- If observation is part of a use-case, link hypothesis to it.
- If it's an invariant quirk, link to invariant.

## Metric

- `hypotheses_per_pattern`: target ≥ 3 per significant pattern.
- `hypothesis_diversity`: pairwise semantic distance ≥ 0.3.
- `predictions_per_hypothesis`: target ≥ 1.
- `patterns_with_hypotheses / total_patterns`: target ≥ 0.9.

## Dependencies

- Forgeplan: `hypothesis` kind, relations `infers_from`, `contradicts`.
- C1, C2, C4 for context.
- LLM with reasoning capability.
- (Later) C6 for triangulation.

## Integration with autoresearch

Via `/autoresearch:reason --mode intent`:
- Uses reason's isolated multi-agent generate→critique→synthesize→judge loop.
- Personas replace default generic judges.
- Output is hypothesis artifacts instead of consensus report.

## Prompt template

See `references/infer-adi.md` (full prompt with personas).

## Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| Hypotheses collapse to one | Similarity between alternatives > 0.8 | Retry with stronger diversity constraint; use Devil's Advocate |
| All hypotheses are wrong | All predictions fail in triangulation | Trigger re-infer with new personas |
| Hypothesis is actually factum | No genuine uncertainty | Detect in post-hoc review; move to factum tier |
| Too many patterns, overwhelms | > 500 patterns in one run | Priority queue: start with highest frequency + highest domain impact |

## Example

See `examples/tripsales-scenario-sample.md` for one domain's hypotheses.

## Testing

Fixture: known pattern with intentionally ambiguous code → expect 3+ distinct hypotheses.

## Version history

- v1.0.0 — initial design.
