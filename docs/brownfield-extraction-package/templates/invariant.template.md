---
kind: invariant
id: INV-{{auto}}
statement: "{{single sentence — the rule}}"
domain: "{{bounded_context}}"
category: "{{authorization | state_transition | referential_integrity | temporal | financial | data_validation}}"
scope: "{{always | precondition | postcondition | event_handling}}"
code_guards:
  - path: "{{file:symbol}}"
    snippet: "{{exact guard expression}}"
violation_consequence: "{{what happens if broken — both system and business view}}"
rationale: "{{why this rule exists — business reason, not technical}}"
related_invariants: []
affected_use_cases: []
verification:
  source: "{{inferred_from_code | domain_owner | external_doc}}"
  confidence: "{{verified | strong-inferred | inferred | speculation}}"
  evidence_refs: []
  last_verified: "{{YYYY-MM-DD}}"
bounded_context: "{{domain}}"
lifecycle_state: "draft"
---

# {{INV-ID}} — {{short name}}

## Statement
{{One sentence, plain English/Russian. The rule itself, nothing more.}}

## Rationale
{{Why this rule exists — business reason. Not "because code has if statement" but "because business requires X".}}

## Scope
{{When it holds: always / at triggering event / after completion / during event handling.}}

## Violation consequence
- **System**: {{error code, rollback, no-op, etc.}}
- **Business**: {{what happens to the real-world process}}

## Covered by
- Use cases: {{UC-XXX list}}
- Scenarios: {{SC-XXX list}}

## Related invariants
- {{INV-XXX — how related (refines, refined by, depends on)}}

## Contradictions
{{None, OR list conflicting invariants with note how to resolve.}}

## Evidence
- Code guard reference: {{EVID-XXX}}
- Interview confirmation: {{EVID-XXX}} (if verified)

{{Speculation warning if applicable: wrap in <!-- confidence:speculation -->...<!-- /confidence --> blocks.}}
