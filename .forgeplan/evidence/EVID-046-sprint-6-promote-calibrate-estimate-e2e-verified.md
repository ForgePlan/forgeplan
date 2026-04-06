---
depth: tactical
id: EVID-046
kind: evidence
links:
- target: EPIC-001
  relation: informs
status: active
title: Sprint 6 — promote + calibrate-estimate E2E verified
---

# EVID-046: Sprint 6 E2E Verification

## Structured Fields

evidence_type: test
verdict: supports
congruence_level: 3

## Result

E2E verification on real workspace (v0.12.0 release binary):

promote:
- `remember "Test memory" → mem-test-memory-for-promote-e2e` created
- `promote mem-xxx --kind note → NOTE-034` created, memory deleted
- Error: promote non-memory → "not found" (correct)
- Error: promote to memory → "Cannot promote memory to memory" (correct)
- Error: promote non-existent → "not found" (correct)

calibrate-estimate:
- `calibrate-estimate PRD-022 --actual-hours 8` → ratio 0.06x, accuracy 6%
- Error: NaN → "must be positive finite number" (correct)
- Error: 0 → rejected (correct)
- Error: negative → clap rejects (correct)

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| Sprint 6 | informs |


