# PRD-006: Smart Routing v2

## Problem

Текущий `forgeplan route` — чистый LLM prompt. Требует API key, стоит денег, медленный (2-5 сек), недетерминированный (разные ответы на тот же input). Для core workflow routing должен быть мгновенным, offline, предсказуемым. LLM может объяснять решение, но не принимать его.

## Goals

- Rule-based routing: детерминированный, instant, offline
- LLM используется ТОЛЬКО для --explain (enrichment, не решение)
- Выходной формат: depth + pipeline + triggers + confidence

## Out of Scope

- Custom user rules (v2 — config-based triggers)
- Machine learning on project history

## Target Users

AI агенты (Claude Code) и разработчики использующие Forgeplan для structured workflow.

## Functional Requirements

- [x] FR-001: Rule engine из DEPTH-CALIBRATION.md decision tree (8 keyword triggers + structural signals)
- [x] FR-002: Pipeline suggestion — Tactical→nothing, Standard→PRD+RFC, Deep→PRD+Spec+RFC+ADR
- [x] FR-003: `forgeplan route` works WITHOUT LLM (instant, offline)
- [x] FR-004: Optional --explain flag calls LLM for human-readable reasoning
- [x] FR-005: Output includes: depth, pipeline, triggers matched, confidence

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-001 | Parent epic | Active |
