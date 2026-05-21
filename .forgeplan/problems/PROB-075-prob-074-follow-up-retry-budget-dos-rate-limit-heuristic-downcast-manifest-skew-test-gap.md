---
depth: tactical
id: PROB-075
kind: problem
links:
- target: PROB-074
  relation: informs
- target: ADR-003
  relation: informs
- target: PROB-072
  relation: informs
- target: PROB-073
  relation: informs
status: draft
title: PROB-074 follow-up — retry budget, DoS rate-limit, heuristic downcast, manifest-skew test gap
---

# PROB-075: PROB-074 follow-up — retry budget, DoS rate-limit, heuristic downcast, manifest-skew test gap

## Signal

Adversarial audit of the PROB-074 fix (2 agents, 12 findings — 1 CRITICAL + 4 HIGH closed inline) deferred 4 findings as design-level or perf-tuning work that's out of scope for the initial stale-handle recovery patch.

## Deferred findings

### F-2 — `with_retry_on_stale` has no retry budget or backoff

`with_retry_on_stale` retries exactly once with no delay. A second stale event in rapid succession (e.g. two concurrent reindex runs) is propagated raw with no exponential backoff or jitter. A workspace under steady reindex churn surfaces flaky `Not found` errors despite the retry path existing.

**Mitigation candidates**:
- Bounded retry loop (N=3) with exponential backoff (100ms, 250ms, 500ms)
- Typed `MutationError::RetryExhausted` so MCP can emit `_next_action: "wait 2s and retry"` instead of raw `internal_error`

### F-3 — `refresh()` DoS amplification, no rate limit

Each MCP read on a stale handle now triggers `refresh()` which calls `checkout_latest` on 3 mandatory + 2 optional tables (5 manifest reads). A local attacker running `while true; do forgeplan reindex; done` makes every MCP call do 5× extra manifest reads. For a dashboard polling every 2s with 5 concurrent agents, that's ~750 manifest reads/min vs ~30 baseline.

**Mitigation candidates**:
- Token-bucket rate limiter on `refresh()` (1 per 250ms per `LanceStore`)
- Debounce: if `last_refresh < 100ms ago`, skip refresh and just retry the op

### F-4 — `is_stale_manifest_error` string-match heuristic

Current detector matches `"Not found:"` + (`.lance/data/` OR `.lance/_versions`). False-positive risk: if an Arrow projection error surfaces an artifact body containing `.lance/data/` (this very PROB body does — it's a legitimate technical reference), a downstream column-mismatch could spuriously trigger refresh-and-retry, masking real bugs.

**Mitigation candidates**:
- Downcast to `lancedb::Error` (which IS a direct dependency, no version-skew concern) and match on the concrete `lance_core::Error::NotFound` variant
- Add an anchored prefix check: `"Not found: "` followed immediately by `/` or `<HOME>` (URI form only)

### F-6 — Integration test reproduces drop-recreate, not true manifest-version skew

`stale_handle_auto_recovers_after_external_reindex` uses `rm -rf lance && LanceStore::init` to simulate the staleness. That **does** invalidate fragment UUIDs, but the real PROB-074 failure mode is more subtle: `forgeplan reindex` writes a NEW manifest version alongside old fragments and atomically swaps — old fragments remain on disk for the GC window. The test does not exercise that "new manifest, old fragments still on disk" case.

**Mitigation candidates**:
- Add a test using `LanceStore::open` + lancedb compaction or an equivalent re-manifest operation
- If the lance API doesn't surface that publicly from a test, document the gap explicitly

## Constraints

- MUST NOT regress the PROB-074 closure (currently 3067/0 tests with audit-pack patches landed)
- MUST keep retry path optional — perf-pure happy path stays zero-overhead
- MAY change the `LanceStore::refresh()` contract if a cleaner design emerges (it's a young surface)

## Acceptance Criteria

- [ ] F-2: bounded retry with backoff + typed `RetryExhausted` error variant
- [ ] F-3: rate-limit or debounce on `refresh()` to bound the amplification factor
- [ ] F-4: heuristic replaced by `lancedb::Error` downcast (or pinned anchored regex)
- [ ] F-6: test exercising manifest-version skew (no `rm -rf`)
- [ ] Audit re-run: zero CRITICAL/HIGH findings on the follow-up sweep

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-074 | informs (this is the explicit follow-up bucket) |
| ADR-003 | informs (lance index is the layer that fails — markdown-first invariant unaffected) |
| PROB-072 | informs (same MCP daemon surface) |
| PROB-073 | informs (rate-limit on refresh ties into per-call latency budget) |





