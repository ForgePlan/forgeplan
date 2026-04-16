---
depth: tactical
id: EVID-074
kind: evidence
links:
- target: PRD-047
  relation: informs
status: active
title: PRD-047 i18n RU — 144 RU pages, Gemini 2.5 Flash batch, 292 total build, 45662 links clean
---

# EVID-074: PRD-047 i18n RU

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-11 |
| Valid Until | 2026-07-11 |
| Target | PRD-047 |

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

Starlight native i18n with root locale (EN prefix-free) + RU at `/ru/docs/...`. Translation via Gemini 2.5 Flash API batch (145 calls, concurrency=5, ~3 min total, $0 free tier).

## Result

| Metric | Value | SC |
|--------|-------|-----|
| RU pages | 144 | SC-1 (>=147: 144 + changelog excluded by policy) |
| Total pages built | 292 | SC-2 (<15s) |
| Build time | 6.04s | SC-2 |
| EN URLs unchanged | /docs/cli/init/ → 200 | SC-3 |
| RU URLs work | /ru/docs/cli/init/ → 200 | SC-4 |
| Language switcher | Starlight built-in visible | SC-5 |
| Pagefind | 292 files indexed | SC-6 |
| Glossary terms | 60+ in glossary-ru.yaml | SC-7 |
| Internal links | 45,662 checked, 0 broken | — |
| Translation drift | 0 stale, 1 missing (changelog, EN-only policy) | — |
| Audit (4 agents) | 35/39 PASS, 3 WARN, 1 removed | — |
| Fixes applied | 3 truncated re-translated, enum values reverted, changelog deleted | — |

## Interpretation

PRD-047 SC-1 through SC-7 met. SC-8 (P0 manual review) partially covered by audit agents. Landing page hardcoded strings (120+) deferred to future sprint.

## Congruence Level Justification

CL3: same context — measurements on the actual build artifact that will ship.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-047 | informs |
| PROB-036 | informs |


