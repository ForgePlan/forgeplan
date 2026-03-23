# RFC-001: FPF Engine — Core Module Architecture

## Summary

Модуль fpf/ в forgeplan-core реализует metrics aggregation, bounded context detection и explore-exploit suggestions. Routing вынесен в отдельный модуль routing/. F-G-R scoring — в scoring/fgr.rs. Всё rule-based, без LLM.

## Motivation

PRD-002 определяет ЧТО нужно: quality scoring, routing, context detection. RFC-001 отвечает на КАК: модульная архитектура с чётким разделением ответственности. Ключевое решение — разделить FPF на три независимых модуля (fpf/, routing/, scoring/) вместо одного монолита, чтобы каждый модуль тестировался и эволюционировал отдельно.

## Architecture (реализовано)

```
crates/forgeplan-core/src/
├── fpf/
│   ├── mod.rs          ← FpfDashboard: aggregates contexts + scores + actions
│   ├── contexts.rs     ← BFS connected-component detection, cohesion metric
│   └── explore.rs      ← Explore/Investigate/Exploit classification
├── routing/
│   ├── mod.rs          ← route(description) → RoutingResult
│   ├── signals.rs      ← 8 keyword triggers + structural signals
│   ├── rules.rs        ← compute_depth (max rank), compute_confidence
│   └── pipeline.rs     ← depth → artifact pipeline mapping
└── scoring/
    ├── reff.rs         ← R_eff = min(evidence_scores), weakest link
    ├── fgr.rs          ← F-G-R: Formality + Granularity + Reliability
    ├── decay.rs        ← Evidence decay impact report
    └── evidence.rs     ← Parse structured fields from evidence body
```

## Implementation Phases

- [x] **G.1** routing/ — rule-based depth calibration (8 keyword triggers, 6 structural signals)
- [x] **G.2** scoring/fgr.rs — F-G-R scoring per artifact (geometric mean, grades A-F)
- [x] **G.3** fpf/contexts.rs — bounded context detection via BFS on link graph
- [ ] ~~**G.4** fpf/adi.rs — structured ADI~~ — deferred (не реализовано)
- [x] **G.5** fpf/explore.rs — explore-exploit suggestions (rule-based thresholds)
- [x] **G.6** CLI: `forgeplan fpf`, `forgeplan route`, MCP tools

## Proposed Solution

### Routing (routing/)
- **Input**: text description (task/feature)
- **Signals**: keyword matching (security→Deep, breaking_change→Deep) + structural heuristics (word count, FR count)
- **Depth**: max(signal.minimum_depth) — conservative, never under-estimates
- **Confidence**: base 0.5 + agreement bonus + count boost, capped at 1.0
- **Pipeline**: depth → artifact kinds (Standard=[PRD,RFC], Deep=[PRD,Spec,RFC,ADR])

### F-G-R Scoring (scoring/fgr.rs)
- **F (Formality)**: % validation rules that pass
- **G (Granularity)**: content density — words (0.3), sections (0.2), checklists (0.2), code (0.15), tables (0.15)
- **R (Reliability)**: R_eff×0.5 + links (0.3) + freshness (0.2)
- **Grade**: cbrt(F×G×R) — geometric mean penalizes imbalance

### Bounded Contexts (fpf/contexts.rs)
- BFS на undirected link graph → connected components
- Cohesion = internal_links / (internal + external)
- Singletons grouped as "Unlinked"

### Explore-Exploit (fpf/explore.rs)
- EXPLORE: R_eff < 0.3, draft, low F-G-R, or orphan
- INVESTIGATE: 0 < R_eff < 0.5 (weak evidence)
- EXPLOIT: R_eff >= 0.7 + fresh evidence

## References

- PRD-002: FPF Reasoning Engine (requirements)
- DEPTH-CALIBRATION.md: original decision tree
- FPF Spec: B.3 Trust Calculus, C.2 F-G-R, C.19 Explore-Exploit
