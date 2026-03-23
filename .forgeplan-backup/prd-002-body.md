# PRD-002: FPF Reasoning Engine

## Summary

Встроить First Principles Framework (FPF) как структурированный reasoning engine в Forgeplan core. FPF Engine = metrics aggregator + rule-based routing + quality scoring + bounded context detection + explore-exploit suggestions. Не заменяет LLM, а дополняет его детерминированной логикой.

## Problem

До FPF Engine Forgeplan был "файловый менеджер с R_eff". `forgeplan reason` = LLM prompt с ADI instructions — "FPF-flavored text", не structured reasoning. Пользователь получал markdown blob, а не structured гипотезы с confidence levels. Проблемы:
- Routing зависел от LLM — медленно, не offline, недетерминированно
- Нет multi-dimensional quality scoring — только R_eff (одно число)
- Нет автоматической архитектурной карты проекта (bounded contexts)
- Нет рекомендаций "что делать дальше" на основе evidence gaps

## Goals

- Детерминированный rule-based routing без LLM (offline, instant, reproducible)
- Multi-dimensional quality scoring через F-G-R (Formality, Granularity, Reliability)
- Автоматическое обнаружение bounded contexts из link graph
- Explore-Exploit suggestions: что исследовать, что использовать, основываясь на evidence coverage
- Единый FPF Dashboard (`forgeplan fpf`) — полная картина проекта

## Target Users

- AI агент (MCP) — получает structured quality data для принятия решений
- Разработчик — `forgeplan route "task"` для мгновенного определения depth и pipeline
- Архитектор — `forgeplan fpf` для обзора bounded contexts и quality grades

## Functional Requirements

- [x] FR-001: FPF Router — rule-based depth calibration (8 keyword triggers, 6 structural signals, confidence scoring)
- [x] FR-002: F-G-R Scoring — Formality (validation pass rate) + Granularity (content density) + Reliability (R_eff + links + freshness), combined via geometric mean
- [x] FR-003: Bounded Context detection — connected-component analysis on link graph with cohesion metric
- [x] FR-004: Explore-Exploit suggestions — rule-based: R_eff < 0.3 = explore, R_eff >= 0.7 = exploit, orphans = explore
- [x] FR-005: FPF Dashboard — `forgeplan fpf` показывает contexts, quality grades, next actions, pipeline status

## Non-Functional Requirements

- NFR-001: FPF Engine работает без LLM (rule-based core, LLM = optional enrichment)
- NFR-002: Не ломает existing commands (backward compatible)
- NFR-003: Каждая фича реализована incremental

## Non-Goals

- Desktop UI для FPF (Phase 5)
- Structured ADI (Abduction → Deduction → Induction) как JSON output — deferred
- Ethics module (FPF Part D) — too complex для MVP
- UTS (Unified Type System) — vocabulary unification
- Dynamic signal weights / ML-trained routing

## Related Artifacts

- EPIC-001: родительский Epic
- PRD-006: Smart Routing v2 (routing = FR-001 of FPF Engine)
- PRD-007: Lifecycle (F-G-R scoring helps lifecycle decisions)
- RFC-001: FPF Engine module architecture

## Implementation Notes

Реализовано в `crates/forgeplan-core/src/`:
- `fpf/mod.rs` (~155 LOC): FpfDashboard struct, dashboard() async fn
- `fpf/contexts.rs` (~80 LOC): BFS connected-component analysis, cohesion metric
- `fpf/explore.rs` (~80 LOC): explore/investigate/exploit classification by R_eff thresholds
- `routing/` (~660 LOC): signals.rs (keyword triggers), rules.rs (depth computation), pipeline.rs (artifact pipeline)
- `scoring/fgr.rs` (~245 LOC): F-G-R scoring with geometric mean, grade mapping A-F
