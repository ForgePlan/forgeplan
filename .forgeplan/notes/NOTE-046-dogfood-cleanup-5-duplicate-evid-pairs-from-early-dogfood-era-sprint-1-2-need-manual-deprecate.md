---
depth: tactical
id: NOTE-046
kind: note
links:
- target: PROB-034
  relation: informs
status: active
title: Dogfood cleanup — 5 duplicate EVID pairs from early dogfood era (Sprint 1-2) need manual deprecate
---

# NOTE-046: Dogfood cleanup — 5 duplicate EVID pairs

Found during v0.17.0 final dogfood audit (2026-04-08) via `forgeplan health`.
PRD-043 duplicate detection (Sprint 13.1) flagged 5 pairs of evidence
artifacts at 100% similarity, originating from very early dogfood
sprints (1-2) when evidence was created twice (test fixture + real ingest).

**These are NOT a v0.17.0 regression** — they predate Sprint 13.1.
PRD-043 catches them now but cannot retroactively deduplicate past data.

## Pairs to clean up

```
EVID-001 ↔ EVID-003 (100%) — "Dogfood lifecycle test"
EVID-002 ↔ EVID-004 (100%) — "Health Dashboard verified"
EVID-005 ↔ EVID-007 (100%) — "Smart Routing v2 verified"
EVID-006 ↔ EVID-008 (100%) — "FPF Engine verified"
EVID-009 ↔ EVID-010 (100%) — "Journal and Validation v2 verified"
```

## Cleanup recipe

For each pair, pick the **earlier** (lower-numbered) artifact as canonical
and deprecate the later one with a `superseded-by` link:

```bash
forgeplan deprecate EVID-003 --reason "duplicate of EVID-001 — early dogfood double-create"
forgeplan link EVID-003 EVID-001 --relation superseded_by
# repeat for 002↔004, 005↔007, 006↔008, 009↔010
```

Then verify: `forgeplan health` should show "Possible duplicates: 0".

## Why a NOTE not a PROB

This is **dogfood cleanup**, not a product bug. PRD-043 detection is
working correctly — it found real duplicates. The fix is data cleanup,
not code change. No user is affected (they don't have Sprint 1-2 dogfood
data).

If during cleanup we find that `forgeplan deprecate` or `forgeplan link
--relation superseded_by` doesn't work as expected, **that** would
become a PROB. Otherwise this is a 5-minute manual pass.

## Acceptance

- All 5 pairs deprecated with explicit reason and superseded_by link
- `forgeplan health` shows 0 duplicate pairs
- Tree view doesn't repeat the same evidence on parent artifacts

## Related

| Artifact | Relation |
|----------|----------|
| PRD-043 | informs (duplicate detection that flagged these) |
| EVID-058 | sibling (Sprint 13.1 implementation) |
| EPIC-003 | context (found in v0.17.0 final audit) |
| NOTE-047 | sibling (other dogfood cleanup task — false-active stubs) |

