---
depth: tactical
id: PROB-032
kind: problem
status: active
title: Search score display breakdown shows all 0.0 when total score is non-zero
---

# PROB-032: Search score breakdown lies about components

## Signal

```
$ forgeplan search "api error"
Found 9 result(s) for "api error":

  0.57  PRD-004 [prd|draft] "Error handling for API endpoints"
        kw=0.0 sem=0.0 r=0.0 g=0.0       ← all components zero
  0.02  PRD-002 [prd|draft] "Authentication OAuth2 system"
        kw=0.0 sem=0.0 r=0.0 g=0.0
  ...
```

Total score is 0.57 for PRD-004, but the breakdown `kw=0.0 sem=0.0
r=0.0 g=0.0` shows all components as zero. **Zero + zero + zero +
zero ≠ 0.57**. The display is lying to the user about score
composition.

## Repro

```bash
cd $(mktemp -d)
forgeplan init -y
forgeplan new prd "Error handling for API endpoints"
forgeplan new prd "Something unrelated"
forgeplan search "api error"
# Check that the breakdown line shows zero components but total
# score is non-zero
```

## Root cause hypothesis

PRD-039 Smart Search v2 (Sprint 13.2) adds multiple scoring
components: BM25 (kw), semantic (sem), R_eff boost (r), granularity
boost (g). Each result gets per-component scores and a total.

The display layer reads component scores from `SmartSearchResult`
struct but something in the computation path doesn't populate them
for the "single-token match" case, while still producing a non-zero
total via a different aggregation.

Two candidate bugs:
1. Display reads wrong fields (r_eff_boost instead of bm25_score)
2. Computation doesn't write components to struct, total is
   synthesized separately
3. Normalization zeroes components but preserves total

Need code investigation in `crates/forgeplan-core/src/search/`.

## Constraints

- Either fix breakdown to show true components (each non-zero when
  contributing), or remove the breakdown line entirely
- Must not change total score ranking
- Must not regress existing search tests

## Acceptance Criteria

1. For a query that matches primarily via BM25, the `kw=X` component
   shows a non-zero value that contributes to the total
2. Sum of displayed components approximately equals the total score
   (within rounding)
3. If no sensible breakdown can be computed, remove the breakdown
   display line entirely rather than show lies
4. Existing `cli_fpf_search_*` and search tests still pass

## Impact

**MEDIUM** — UX confusion, not data corruption. Users trying to
understand ranking see contradictory info. Undermines trust in
search quality.

## Blast Radius

- CLI `forgeplan search` output only
- MCP `forgeplan_search` output if it exposes breakdown too

## Reversibility

HIGH — pure display code fix OR removal of misleading line.

## Related

| Artifact | Relation |
|---|---|
| PRD-039 | informs (Sprint 13.2 BM25 introduced breakdown) |
| PROB-030 | sibling (both BM25 regressions from quality audit) |
| NOTE-048 | sibling |

