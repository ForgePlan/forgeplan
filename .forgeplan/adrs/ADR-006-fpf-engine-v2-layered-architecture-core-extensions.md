---
depth: standard
id: ADR-006
kind: adr
links:
- target: RFC-001
  relation: based_on
status: active
title: FPF Engine v2 — Layered Architecture (Core + Extensions)
---

# ADR-006: FPF Engine v2 — Layered Architecture (Core + Extensions)

## Context

FPF Engine (794 LOC, 4 файла) имел 3 проблемы:
1. Все scoring параметры захардкожены — каждый пользователь требует правки кода
2. ADI reasoning одноразовый — результат теряется после сессии
3. Модули не связаны общей моделью данных

RFC-001 исследовал 3 варианта архитектуры. ADI reasoning (NOTE-037) подтвердил выбор с High confidence.

## Decision

**Selected**: Option C — Layered Architecture (Core + Extensions)

**Why Selected**: Баланс между "делаем правильно" и "не over-engineer". Core = pure functions (testable без LanceDB). Extensions = добавляются инкрементально. В отличие от Option A (только config) — полная extensibility. В отличие от Option B (всё сразу) — инкрементальная доставка и простой rollback.

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| A: Config-only | Rejected | Только thresholds в config, правила не extensible. ~300 LOC, быстро, но не масштабируется |
| B: Unified Model + Rule Engine | Rejected | Всё за один рефактор. ~800 LOC, high migration risk, нет инкрементальной доставки |
| **C: Layered Core + Extensions** | **Chosen** | Core (pure) + Extensions (incremental). ~600 LOC, medium risk, каждая фаза независимо rollback-able |

## Consequences

### Positive
- Scoring параметры configurable через config.yaml — работает для любого пользователя
- ADI results trackable как structured data (AdiRecord)
- Core testable без I/O — 37 unit tests, ~80% coverage
- Phase 2/3 добавляются без breaking changes
- MCP и CLI используют одни и те же weights из config

### Negative (trade-offs)
- Два места с reliability логикой (trust.rs + fgr.rs) — DRY debt до Phase 2
- Больше файлов в fpf/ (4 → 8+ после Phase 2)
- FpfConfig нужно загружать в каждом call site

### Risks
- Rule engine YAML syntax (Phase 2) может быть неудобен
- FpfContext on-the-fly computation для 1000+ артефактов — latency

## Invariants

- **R_eff = min(evidence_scores)** — weakest link principle НИКОГДА не average
- **Default config = текущее поведение** — None/отсутствие fpf секции = hardcoded defaults
- **Core не зависит от I/O** — trust.rs, adi.rs, model.rs = pure functions
- **FpfConfig::validate()** вызывается при загрузке — NaN/Infinity/negative отклоняются

## Evidence Requirements

- [x] Custom weights меняют scoring output (verified: PROB-011 Reliability 0.30 → 0.13)
- [x] 790 tests pass, 0 failures
- [x] 4 раунда аудита (8 agents), 6 HIGH fixed, 0 open
- [x] Все CLI + MCP commands smoke tested
- [ ] Phase 2: rule engine extensibility (Sprint 12)
- [ ] Phase 3: dashboard performance with 1000+ artifacts

## Valid Until

**Дата**: 2027-04-06 (1 год)

**Обоснование TTL**: архитектурное решение, не меняется часто. Пересмотр при v3.0 или значительном расширении scope.

**Refresh Triggers**:
- Если Phase 2 rule engine оказывается неудобен → пересмотреть подход
- Если появляются >3 duplicate reliability logic paths → consolidate

## Pre-conditions (чеклист ДО реализации)

- [x] RFC-001 shaped и validated
- [x] ADI reasoning подтвердил Option C (High confidence)
- [x] Existing tests не сломаны (790 pass)

## Post-conditions (Definition of Done)

- [x] fpf/core/ модуль: config.rs, trust.rs, adi.rs, model.rs
- [x] FpfConfig wired в CLI (score, fgr, context, dashboard) и MCP (score)
- [x] Config templates в init + текущий config.yaml
- [x] AdiRecord wiring в forgeplan reason --save
- [x] FpfConfig::validate() вызывается при загрузке
- [x] 4 раунда аудита, все HIGH fixed
- [ ] Phase 2: ext/rules.rs, ext/knowledge.rs (Sprint 12)
- [ ] Phase 3: dashboard refactor, MCP tools (Sprint 12)

## Admissibility

- NOT: менять формулу R_eff (min, не average) без нового ADR
- NOT: добавлять I/O зависимости в fpf/core/
- NOT: хардкодить новые пороги — всё через FpfConfig
- NOT: создавать альтернативный scoring path без ссылки на этот ADR

## Rollback Plan

**Triggers**:
- Phase 2 rule engine неудобен или performance degradation
- Core модуль создаёт confusion с legacy scoring

**Steps**:
1. Старые файлы (fpf/mod.rs, explore.rs, contexts.rs) не удалены — они работают
2. Убрать fpf/core/ из mod.rs
3. Вернуть hardcoded значения в fgr::compute_reliability
4. Git revert последних коммитов

**Blast Radius**: только fpf/ модуль и scoring weights. Артефакты и data не затрагиваются.

## Weakest Link

Дублирование reliability логики между trust.rs и fgr.rs. Если одна изменится без другой — scores diverge. Планируем consolidation в Phase 2.

## Affected Files

| File | Change |
|------|--------|
| crates/forgeplan-core/src/fpf/core/*.rs | NEW — core module (4 files) |
| crates/forgeplan-core/src/scoring/fgr.rs | MODIFIED — parameterized weights |
| crates/forgeplan-core/src/config/types.rs | MODIFIED — FpfConfig field |
| crates/forgeplan-core/src/workspace/init.rs | MODIFIED — config templates |
| crates/forgeplan-cli/src/commands/*.rs | MODIFIED — config loading (5 files) |
| crates/forgeplan-mcp/src/server.rs | MODIFIED — fpf_weights loading |

## AI Guidance

> Правила для AI-агентов при работе с FPF Engine.

- Все scoring параметры — через FpfConfig, НИКОГДА hardcoded magic numbers
- Новый код в fpf/ → в core/ (pure) или ext/ (I/O), не в root
- При изменении trust calculus — обновить ОБА trust.rs и fgr.rs (до consolidation)
- `forgeplan reason --save` создаёт AdiRecord — не менять формат без миграции
- FpfConfig::validate() вызывается автоматически — не обходить

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| RFC-001 | RFC | based_on |
| EPIC-002 | Epic | child_of |
| EVID-055 | Evidence | informed_by |
| NOTE-037 | Note | informed_by (ADI reasoning) |
| PROB-021 | Problem | addresses |


