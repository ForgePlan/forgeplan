---
depth: tactical
id: EVID-043
kind: evidence
links:
- target: PROB-017
  relation: informs
status: active
title: PROB-017 router alternatives — 9 tests, 2-agent audit, 2 HIGH fixed
---

# EVID-043: PROB-017 router alternatives — 9 tests, 2-agent audit, 2 HIGH fixed

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-02 |
| Valid Until | 2026-07-02 |
| Target | PROB-017 |

## Structured Fields

evidence_type: test
verdict: supports
congruence_level: 3

## Measurement

- 9 new unit tests for `generate_alternatives()` and `RouteAlternative`
- 2-agent audit (security+correctness, rust patterns)
- 725 total tests pass, 0 failures, 0 warnings
- CLI visual verification: Tactical, Standard, Deep all show correct alternatives

## Result

- All 4 acceptance criteria from PROB-017 met:
  1. CLI shows primary + 2 alternatives with reasoning
  2. MCP includes `_alternatives` array
  3. Each alternative has contextual reasoning
  4. All existing tests pass unchanged (1 test updated for new output format)
- 2 HIGH audit findings fixed (stale alternatives after downgrade, reflexive loop)

## Interpretation

PROB-017 fully resolved. Router now provides transparent depth selection with alternatives, enabling AI agents to evaluate trade-offs programmatically.

## Congruence Level Justification

CL3: internal tests on the same codebase, same context as the problem.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-017 | informs |


