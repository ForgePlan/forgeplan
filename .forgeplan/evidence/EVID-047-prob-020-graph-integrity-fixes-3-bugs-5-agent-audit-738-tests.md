---
depth: tactical
id: EVID-047
kind: evidence
links:
- target: PROB-020
  relation: informs
status: active
title: PROB-020 graph integrity fixes — 3 bugs, 5-agent audit, 738 tests
---

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Evidence

### Bugs Fixed
1. **BUG-1:** `blocked`/`order` now use `resolved_ids` (active+deprecated+superseded). Blocked: 14 → 5.
2. **BUG-2:** `delete` cascades all relations via `delete_relations_for_artifact()`. Phantom PROB-013 cleaned up.
3. **BUG-2b:** `unlink` works for deleted source artifacts (resilient lookup via `get_all_relations()`).

### Audit Results
- 5-agent panel: logic (7/10), rust (8/10), security (7/10), arch (7/10), test (6/10)
- Verdict: APPROVE_WITH_FIXES
- 2 critical (CC1 parameter rename, CC2 case-sensitivity) → both fixed
- DRY helper extracted to `common::resolved_ids()`

### Test Results
- 738 tests total (+7 new), 0 failures, 0 warnings
- New tests: deprecated_does_not_block, superseded_does_not_block, draft_still_blocks, mixed_draft_and_deprecated_deps, stale_blocks_by_design, delete_relations_for_artifact_cascades, delete_relations_for_artifact_returns_zero_when_none

### Files Changed
7 files, ~150 LOC



