---
depth: tactical
id: EVID-068
kind: evidence
links:
- target: PROB-030
  relation: informs
status: active
title: PROB-030 BM25 prefix fallback verified (v0.17.2)
---

# EVID-068: PROB-030 BM25 prefix fallback verified (v0.17.2)

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-09 |
| Valid Until | 2026-07-08 |
| Target | PROB-030 (hotfix target) |

<!-- Fill in the Structured Fields section below for R_eff scoring.
     These fields are REQUIRED for correct R_eff calculation.
     evidence_type: measurement | test | benchmark | audit
     verdict: supports | weakens | refutes
     congruence_level: 0 | 1 | 2 | 3 (CL3=same context, CL0=opposed context)
-->

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

**What**: query `auth` on corpus with PRD-001 "Authentication OAuth2 system"
returns 2 exact results (0.80) — was 0 on v0.17.1 baseline.

**How**: added `let kw = keyword_score(record, query); let keyword_channel = bm25_norm.max(kw);` in
`crates/forgeplan-core/src/search/smart.rs` combined_score computation.

**Corpus**: `/tmp/fp-e2e` with 4 PRDs, 1 RFC, 1 ADR.

## Result

- T1 `authentication` → 2 results (PRD-001 @ 0.80, RFC-001 @ 0.80) ✓
- T2 `auth` (prefix, PROB-030) → 5 results incl. both Authentication artifacts at 0.80 ✓
- T3 `OAu` (prefix) → PRD-001 @ 0.80 ✓
- T4 `error handling` (multi-word) → PRD-002 @ 0.80 ✓
- T5 `PAYMENT` (case) → PRD-003 @ 0.80 ✓
- Regression tests: smart_search_prefix_query_falls_back_to_substring,
  smart_search_exact_token_still_wins_over_prefix (both green)

## Interpretation

BM25 prefix fallback landed correctly — `search auth` now finds Authentication artifacts via substring channel without demoting exact-token BM25 hits. Users can search as they type, grep-like behavior.

## Congruence Level Justification

<!-- Почему выбран именно этот CL:
     CL3: тот же контекст, внутренний тест (penalty 0.0)
     CL2: похожий контекст, related project (penalty 0.1)
     CL1: другой контекст, внешняя документация (penalty 0.4)
     CL0: противоположный контекст (penalty 0.9) -->

CL3 — same-context T1 evidence: tests written and executed locally against the exact code they verify. No context shift.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-068 | informs |



