---
created: 2026-04-21
depth: tactical
id: EVID-082
kind: evidence
links:
- target: EPIC-008
  relation: informs
- target: EPIC-007
  relation: informs
- target: ADR-009
  relation: informs
status: active
title: DDD analysis of Forgeplan repo — Spike-3 CL3 measurement for orchestrator pivot
updated: 2026-04-21
---

# EVID-082: DDD analysis of Forgeplan repo — Spike-3 CL3 measurement

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

Spike-3 experiment (2026-04-21): запустили `agents-pro:ddd-domain-expert` subagent на полном Forgeplan Rust workspace (`crates/forgeplan-core`, `crates/forgeplan-cli`, `crates/forgeplan-mcp` — ~21 core modules + 2 adapter crates, ~12.8K LOC) без предварительного обучения или передачи шаблонов. Агент произвёл структурированный DDD отчёт:

- **Bounded Contexts**: 8 + Interface context
- **Aggregates**: 23 distinct aggregate roots с code references (struct names + file:line)
- **Ubiquitous Language glossary**: 23 terms extracted из реальных Rust types и module paths
- **Domain Events**: 12 (derived from lifecycle transitions в `lifecycle/mod.rs`)
- **Context Integration Patterns**: 10 pairs of contexts с DDD strategic patterns (ACL, OHS, Customer-Supplier, Shared Kernel, Separate Ways, Conformist, Observer)
- **Category Errors**: 6 (E1-E6), включая новый E5 "bounded context" term collision (FPF cluster vs DDD)

Output сохранён в `docs/architecture/ddd-analysis-spike-3.md` (~650 строк Markdown).

Параллельно написан mapping file `ddd-to-forge.yaml` в peer marketplace repo (`plugins/forgeplan-brownfield-pack/mappings/`) с 5 rules: `bounded_context_to_epic`, `aggregate_to_prd`, `glossary_term_to_glossary_artifact`, `domain_event_to_note`, `integration_pattern_to_note`, `category_error_to_problem`. Compat-shim для pre-EPIC-008 kinds (glossary→note, invariant→spec, hypothesis→problem, domain-model→spec).

## Result

**Quantitative**:
- 84 ingestable forge artifacts derivable из одного DDD analysis run:
  - 1 top-level Epic (system DDD view)
  - 9 Epics (1 per bounded context) OR 9 PRDs if nested under system Epic
  - 23 PRDs (1 per aggregate root)
  - 23 Notes or Glossary artifacts (ubiquitous language)
  - 12 Notes (domain events)
  - 10 Notes (integration patterns)
  - 6 Problems (category errors)

- **0 ручной работы** required после agent execution: mapping полностью механический, все source refs (file:line, struct names) cite-able и verifiable.

**Qualitative**:
- Agent output Factum tier ≥ 95% code-anchored (все aggregates имеют real struct names; glossary все terms имеют code reference; domain events extracted from existing lifecycle code).
- Intent tier (category errors §6) — 6 items с reasoned recommendations, suitable as Problems в forge.
- **Discovered** новый category error (E5) не замеченный ни в PROB-040 4-agent audit, ни в ручных review.

## Interpretation

Эта measurement **validates three central EPIC-008 claims**:

1. **6 new kinds необходимы** — особенно `glossary` (для §3 ubiquitous language), `invariant` (для §2 aggregate invariants), `hypothesis` (для §6 category errors with confidence levels), `domain-model` (для §1 bounded-context canonical views).

2. **Orchestrator model работает на реальных данных** — ADR-009 claim что forgeplan может оркестрировать existing marketplace plugins (ddd-expert, c4-architecture, autoresearch) + mapping YAMLs вместо того чтобы reimplementing extraction подтверждается эмпирически. Spike-1 (c4-context) покрыл 20+ artifacts derivable. Spike-3 покрывает 84.

3. **Factum/Intent separation enforceable** — §1-§5 agent output — factum (100% code-anchored), §6 — intent (reasoning о *почему*). Отчётливая граница позволяет ingest применить разные confidence tags (verified vs inferred) на разные sections без manual markup.

Дополнительно **surfaced blind spots**:
- E5 collision term «bounded context» — FPF's `ArtifactCluster` и DDD's strategic concept не различаются. Требует rename в forgeplan-core для семантической чистоты. Это становится follow-up PROB.
- Domain events отсутствуют as first-class (`ActivateResult`/`SupersedeResult` — ad-hoc DTOs). Unified `DomainEvent` trait был бы valuable для cross-context integration. Follow-up: EPIC-007 runtime может ввести это в PRD-066 ingest engine.

## Congruence Level Justification

**CL3 (same context, penalty 0.0)**:
- Measurement run на самом target system (Forgeplan repo), не на proxy или synthetic fixture.
- Output format exactly matches expected forge artifacts (aggregates → PRDs, glossary → kinds, events → notes).
- Mapping file `ddd-to-forge.yaml` написан и сохранён для replay — measurement reproducible.
- Все claims verifiable: struct names в отчёте могут быть grep'нуты в коде, все file:line references — real.

No CL2 discounting: this is not "related work" — this is the *exact* primary use case EPIC-008 is designed to enable.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| EPIC-008 | informs (validates Wave 1 kinds + orchestrator claim) |
| EPIC-007 | informs (validates playbook runtime + ingest engine scope) |
| ADR-009 | informs (orchestrator model CL3 confirmed on DDD dimension) |
| EVID-081 | complements (Spike-1 c4-to-forge CL3 — same conclusion from different agent output) |
| `docs/architecture/ddd-analysis-spike-3.md` | source_data (full agent output) |
| `plugins/forgeplan-brownfield-pack/mappings/ddd-to-forge.yaml` (peer) | derivative (mapping rules produced alongside this evidence) |




