---
depth: tactical
id: EVID-070
kind: evidence
links:
- target: PROB-032
  relation: informs
status: active
title: PROB-032 score breakdown shows real kw component (v0.17.2)
---

# EVID-070: PROB-032 score breakdown shows real kw component (v0.17.2)

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-09 |
| Valid Until | 2026-07-08 |
| Target | PROB-032 (hotfix target) |

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

**What**: `forgeplan search` showed `kw=0.0 sem=0.0 r=0.0 g=0.0` breakdown
while total score was 0.57 — display lied about components.

**How**: auto-fixed as side effect of PROB-030. Once `keyword_channel =
bm25_norm.max(kw)` is passed into combined_score, the real value flows
through to the breakdown display. No separate fix needed.

## Result

- E2E: `forgeplan search "auth"` now shows kw=0.80 (was 0.0) — breakdown honest
- Total score equals sum of non-zero components (within rounding)
- No separate regression test — covered by PROB-030 tests

## Interpretation

Score breakdown is now honest: `kw`, `sem`, `r`, `g` all reflect real values. Covered transitively by PROB-030 assertions — single source parser removes the 'display lies' class of bugs.

## Congruence Level Justification

<!-- Почему выбран именно этот CL:
     CL3: тот же контекст, внутренний тест (penalty 0.0)
     CL2: похожий контекст, related project (penalty 0.1)
     CL1: другой контекст, внешняя документация (penalty 0.4)
     CL0: противоположный контекст (penalty 0.9) -->

CL3 — same-context T1 evidence: transitive coverage proven by the same-session PROB-030 tests that exercise `combined_score` end-to-end.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-070 | informs |



