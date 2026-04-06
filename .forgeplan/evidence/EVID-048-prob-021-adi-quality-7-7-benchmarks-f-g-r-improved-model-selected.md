---
depth: tactical
id: EVID-048
kind: evidence
links:
- target: PROB-021
  relation: informs
status: active
title: PROB-021 ADI quality — 7/7 benchmarks, F-G-R improved, model selected
---

# EVID-048: PROB-021 ADI quality — 7/7 benchmarks, F-G-R improved, model selected

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-04 |
| Valid Until | 2026-07-04 |
| Target | PROB-021 |

## Structured Fields

evidence_type: benchmark
verdict: supports
congruence_level: 3

## Measurement

7 ADI reason runs on real project artifacts (PRD-004, PRD-006, PRD-011, PRD-018, PRD-022, PROB-020, RFC-003) using gemini-3-flash-preview with enriched prompt (metadata context, architecture hint, justified confidence).

Compared 4 models: gemini-3.1-pro-preview, gemini-2.5-pro, gemini-2.5-flash, gemini-3-flash-preview.

## Result

Before (no context, gemini-2.5-pro): F=4 G=2 R=1, 1/3 irrelevant hypotheses, 17.9s
After (enriched, gemini-3-flash-preview): F=5 G=3 R=1, 0/7 irrelevant, 23.3s avg

| Model | Time | JSON OK | Justified | Garbage | Lang |
|-------|------|---------|-----------|---------|------|
| gemini-3-flash-preview | 23s | 7/7 | 21/21 | 0 | 4RU/3EN |
| gemini-3.1-pro-preview | 34s | 7/7 | 21/21 | 0 | RU |
| gemini-2.5-pro | 41s | 7/7 | 21/21 | 0 | RU |
| gemini-2.5-flash | 27s | 0/7 | 21/21 | 0 | EN |

747 unit tests pass, 7 new tests added, 0 failures.

## Interpretation

Enriched ADI prompt eliminates irrelevant hypotheses (G: 2→3) and forces justified confidence (F: 4→5). gemini-3-flash-preview matches Pro quality at 47% less latency. gemini-2.5-flash fails JSON parsing — not suitable.

## Congruence Level Justification

CL3: same project, same artifacts, same codebase — direct measurement on production data.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-021 | informs |


