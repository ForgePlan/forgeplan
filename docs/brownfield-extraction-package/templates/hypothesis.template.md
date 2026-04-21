---
kind: hypothesis
id: HYP-{{auto}}
subject: "{{the question being asked}}"
observation: "{{what was noticed in code}}"
bounded_context: "{{domain}}"
lifecycle_state: "drafted"
candidates:
  - id: "H1"
    statement: "{{explanation 1}}"
    prior_probability: 0.0
    supporting_evidence: []
    refuting_evidence: []
    plausibility_score: 0.0
  - id: "H2"
    statement: "{{explanation 2}}"
    prior_probability: 0.0
    supporting_evidence: []
    refuting_evidence: []
    plausibility_score: 0.0
  - id: "H3"
    statement: "{{explanation 3}}"
    prior_probability: 0.0
    supporting_evidence: []
    refuting_evidence: []
    plausibility_score: 0.0
selected_candidate: null
confidence_rationale: "{{why the selected candidate is current best OR 'awaiting triangulation'}}"
triangulation_sources: []
code_refs:
  - "{{file:line}}"
related_artifacts: []
parked_in: null
verification:
  last_reviewed: "{{YYYY-MM-DD}}"
  days_since_promotion: 0
blocks_artifacts: []
---

# {{HYP-ID}} — {{short subject}}

## Observation
{{What was noticed in the code that prompted this hypothesis? Describe the pattern — status values, branching, naming quirk, unusual structure.}}

## Subject
{{The question being asked: "Why X?"}}

## Candidates

### H1 — {{short name}} (plausibility {{0.0-1.0}})
{{Explanation 1.}}

**Supporting**: {{evidence}}
**Refuting**: {{evidence}}

### H2 — {{short name}} (plausibility {{0.0-1.0}})
{{Explanation 2.}}

**Supporting**: {{evidence}}
**Refuting**: {{evidence}}

### H3 — {{short name}} (plausibility {{0.0-1.0}})
{{Explanation 3.}}

**Supporting**: {{evidence}}
**Refuting**: {{evidence}}

## Selected
{{Selected hypothesis + current confidence level + rationale.}}

## Next action
- [ ] Triangulate (git / docs / comments / naming)
- [ ] Park for Domain Owner interview
- [ ] Escalate as contradiction with {{existing_invariant}}
- [ ] Accept as {{verified | strong-inferred | inferred}}
- [ ] Refute

## Blocked artifacts
- {{list of use-cases / scenarios / invariants that can't be finalized until this is resolved}}
