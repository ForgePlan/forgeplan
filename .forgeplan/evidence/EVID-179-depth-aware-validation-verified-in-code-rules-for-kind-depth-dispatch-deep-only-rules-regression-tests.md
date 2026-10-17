---
depth: tactical
id: EVID-179
kind: evidence
links:
- target: PRD-005
  relation: informs
status: active
title: Depth-aware validation verified in code — rules_for(kind, depth) dispatch + deep-only rules + regression tests
---

---
assigned_number: 179
created: 2026-10-17
predicted_number: 179
slug: evid-depth-aware-validation-verified-in-code-rules-for-kind-depth-dispatch-deep
updated: 2026-10-17
---

# EVID-179: Depth-aware validation verified in code — rules_for(kind, depth) dispatch + deep-only rules + regression tests

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-10-17 |
| Target | PRD-005 |

## Structured Fields

evidence_type: test
verdict: supports
congruence_level: 3

## Measurement

Verified PRD-005 (Depth-Aware Validation) is implemented in the same repo:

- `rules_for(kind, depth)` dispatch — `crates/forgeplan-core/src/validation/rules.rs:26`. Per-depth rule sets for PRD/RFC/ADR.
- Deep-only rules present: Timeline (647), Stakeholders (655), Acceptance Criteria (663), Risk/Risks (673, 1234), rollback (681), Dependencies (713).
- Severity escalation: density/leakage rules upgrade at `Mode::Deep` (387–418).
- Regression suite: `cargo test -p forgeplan-core rules_for --lib` → 13/13 passed, incl. `rules_for_prd_deep_includes_timeline_stakeholders_acceptance`, `rules_for_prd_tactical_returns_base_only`, `rules_for_adr_deep_returns_base_plus_8_with_must_contracts`.
- Live check: `forgeplan validate PRD-087` reports `(depth: Deep)` and applies the deep rule set — PASS with warnings.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-005 | informs |



