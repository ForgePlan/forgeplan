---
depth: deep
id: RFC-001
kind: rfc
links:
- target: PRD-002
  relation: based_on
status: active
title: FPF Engine v2 — configurable trust calculus, ADI tracking, extensible scoring
---

## Progress

```
Phase 1  ████████████████████████  7/7  (100%)
Phase 2  ░░░░░░░░░░░░░░░░░░░░░░░░  0/5  (  0%)
Phase 3  ░░░░░░░░░░░░░░░░░░░░░░░░  0/4  (  0%)
─────────────────────────────────────────────────
TOTAL                               7/16 ( 44%)
```

## Summary

Рефактор FPF Engine из набора захардкоженных модулей (contexts.rs, explore.rs, knowledge.rs) в configurable, extensible engine с единой моделью данных, trackable ADI reasoning и trust calculus доступным через config — чтобы ForgePlan работал не только у автора, а у любого пользователя.

## Motivation

### Проблема

Текущий FPF Engine (794 LOC, 4 файла) — это 3 несвязанных модуля без общей модели:

1. **Всё захардкожено** — R_eff пороги (0.01, 0.5, 0.7), F-G-R веса (0.5, 0.3, 0.2), explore-exploit правила — в коде, не в config. Каждый пользователь потребует изменения кода.

2. **ADI — одноразовый LLM output** — `forgeplan reason` генерирует гипотезы, но результат не хранится как trackable data. Через неделю невозможно ответить "почему мы выбрали этот подход?"

3. **Модули не знают друг о друга** — contexts.rs работает с BFS графом, explore.rs с порогами R_eff, knowledge.rs с markdown файлами. Нет единой модели "FPF Context" с trust scores + ADI history + impact chains.

4. **KB search keyword-only** — "Authentication" vs "Auth" = разные результаты. Семантический поиск есть в LanceDB, но KB его не использует.

### Что будет если НЕ делать

- ForgePlan остаётся internal tool, не продукт — новый пользователь не может настроить scoring под свой проект
- ADI reasoning теряется между сессиями — главная ценность (трекинг решений) не работает
- EPIC-002 O4 ("reasoning fully leverages FPF") не достижим

### Evidence

- RFC-001 stub с 24 марта — 2 недели без заполнения (потребность назрела)
- 39 тестов, ~23% покрытие fpf/ (quality debt)
- knowledge.rs — 0 тестов (0% coverage)
- PROB-021 зафиксировал: ADI генерирует noisy hypotheses из-за missing context

## Goals

- [Actor] can configure trust calculus thresholds (R_eff boundaries, F-G-R weights, CL penalties) через `config.yaml` без изменения кода
- [Actor] can track ADI reasoning history — гипотезы, verdicts, confidence changes хранятся как первоклассные данные, связанные с артефактами
- [Actor] can see bounded context for any artifact — в каком модуле живёт, что затронет изменение
- [Actor] can search FPF KB семантически — vector search вместо keyword matching
- [Actor] can extend explore-exploit rules через config (custom правила, thresholds per artifact type)
- Test coverage fpf/ ≥ 60% (с 23%)

## Non-Goals

- Multi-agent orchestration (EPIC-002 non-goal)
- GUI/TUI для FPF dashboard (Sprint 13-14)
- Custom scoring formulas (plugin system) — extensibility через config, не через code
- Замена LLM provider abstraction (уже в llm/ модуле)
- Per-dimension valid_until expiry (FPF spec, но low priority для v2)

## Options Considered

### Option A: Trait-based engine с FpfConfig

**Description**: Вынести все параметры в `FpfConfig` struct, загружаемый из `config.yaml`. Scoring, explore-exploit, и context detection остаются в тех же файлах, но читают пороги из config. ADI output сохраняется как новый artifact kind `AdiRecord` в LanceDB.

```rust
// config.yaml
fpf:
  thresholds:
    explore_reff: 0.01
    investigate_reff: 0.5
    exploit_reff: 0.7
    exploit_fgr: 0.6
  weights:
    reliability_reff: 0.5
    reliability_links: 0.3
    reliability_freshness: 0.2
  adi:
    max_hypotheses: 5
    kb_sections_limit: 5
    temperature_cap: 0.3

// Rust
pub struct FpfConfig {
    pub thresholds: Thresholds,
    pub weights: Weights,
    pub adi: AdiConfig,
}

impl Default for FpfConfig {
    fn default() -> Self { /* current hardcoded values */ }
}
```

ADI tracking: новая таблица `adi_records` в LanceDB с полями hypothesis_id, artifact_id, confidence, verdict, created_at. `forgeplan reason` автоматически сохраняет. `forgeplan adi-history <id>` показывает эволюцию решений.

**Pros**:
- Минимальный рефактор — существующий код не переписывается, только параметризуется
- Обратная совместимость — default values = текущее поведение
- ADI tracking — новый artifact kind, использует существующую инфраструктуру (LanceDB, scan-import)
- ~300-400 новых LOC

**Cons**:
- Explore-exploit правила остаются 4 if-statements — нельзя добавить новые без кода
- Bounded contexts остаются BFS connected components — нет семантики
- Модули по-прежнему не имеют общей модели данных

### Option B: Unified FPF Model + Rule Engine

**Description**: Ввести единую модель `FpfContext` которая объединяет trust score, bounded context membership, ADI history и impact chain для каждого артефакта. Explore-exploit становится rule engine с declarative правилами в YAML.

```rust
pub struct FpfContext {
    pub artifact_id: String,
    pub trust: TrustScore,        // R_eff + CL + decay + recursive
    pub context: ContextMembership, // cluster_id, cohesion, role
    pub adi_history: Vec<AdiSnapshot>,
    pub impact: ImpactChain,       // downstream artifacts affected
}

pub struct TrustScore {
    pub r_eff: f64,
    pub formality: f64,
    pub granularity: f64,
    pub reliability: f64,
    pub overall: f64,
    pub weakest_link: Option<String>,
}
```

Rule engine:
```yaml
# config.yaml
fpf:
  rules:
    - name: "blind-spot"
      when: { r_eff: "<0.01", status: "draft" }
      action: EXPLORE
      priority: 1
      message: "Draft with no evidence — needs investigation"
    - name: "stale-evidence"
      when: { r_eff: "0.01..0.5" }
      action: INVESTIGATE
      priority: 2
    - name: "ready-to-build"
      when: { r_eff: ">=0.7", fgr: ">=0.6" }
      action: EXPLOIT
      priority: 5
```

**Pros**:
- Единая модель — каждый артефакт имеет полный FPF context в одном месте
- Extensible rules — пользователь добавляет правила без кода
- Impact chain — "если я меняю RFC-001, что сломается?"
- ADI history как часть модели, не отдельный artifact kind

**Cons**:
- ~600-900 новых LOC, значительный рефактор
- Rule engine = мини-язык в YAML, нужен парсер для условий
- Migration risk — текущие данные нужно мигрировать в новую модель
- Over-engineering для solo-first? (но мы решили что нет)

### Option C: Layered Architecture — Core + Extensions

**Description**: Разделить fpf/ на 2 слоя: **Core** (минимальный, стабильный, хорошо протестированный) и **Extensions** (опциональные, configurable). Core содержит trust calculus и ADI data model. Extensions содержит rule engine, impact analysis, KB semantic search.

```
fpf/
├── core/
│   ├── trust.rs      — TrustScore computation (configurable via FpfConfig)
│   ├── adi.rs        — AdiRecord, AdiSnapshot, hypothesis tracking
│   └── model.rs      — FpfContext unified model
├── ext/
│   ├── rules.rs      — Declarative rule engine (YAML)
│   ├── impact.rs     — Downstream impact chain analysis
│   ├── contexts.rs   — Bounded context detection (improved)
│   └── knowledge.rs  — FPF KB with vector search
├── config.rs         — FpfConfig from config.yaml
├── dashboard.rs      — Aggregator (was mod.rs)
└── mod.rs            — Public API
```

Core гарантии:
- TrustScore computation = pure function (no side effects, fully testable)
- AdiRecord = serializable, versionable, linkable to artifacts
- FpfContext = computed from store data + config, not stored

Extensions:
- Rule engine reads YAML, evaluates against FpfContext → actions
- Impact analysis walks dependency graph → returns affected artifacts
- KB search uses existing LanceDB vector infrastructure

**Pros**:
- Чёткое разделение: core стабилен, extensions эволюционируют
- Core testable без LanceDB (pure functions)
- Extensions можно добавлять по одному (incremental delivery)
- ADI model + trust calculus = reusable для других модулей
- ~500-700 новых LOC, но инкрементально

**Cons**:
- Больше файлов, больше module structure
- Нужно решить: FpfContext computed on-the-fly или cached?
- Rule engine всё ещё нужен (как в Option B)

## Trade-off Analysis

| Критерий | A: Config-only | B: Unified Model | C: Layered Core+Ext |
|----------|---------------|------------------|---------------------|
| Complexity | Low (~300 LOC) | High (~800 LOC) | Medium (~600 LOC) |
| Extensibility | Config only | Full (rules+model) | Full (rules+model) |
| Migration risk | Minimal | High (new model) | Medium (incremental) |
| Testability | Same as now | Better (unified) | Best (pure core) |
| Time to deliver | 1-2 дня | 3-5 дней | 2-3 дня (Phase 1), +2 дня (Phase 2) |
| User configurability | Thresholds only | Thresholds + rules | Thresholds + rules |
| ADI tracking | New artifact kind | Part of model | Part of core model |
| Solo-first fit | Good | Over-engineered? | Good (core first) |
| Product scalability | Limited | Full | Full |

## Proposed Direction

**Option C: Layered Architecture** — best balance между "делаем правильно" и "не over-engineer":

1. **Core** (Phase 1) = trust calculus + ADI model + FpfConfig — стабильный фундамент, хорошо протестированный, pure functions
2. **Extensions** (Phase 2) = rule engine + impact + improved contexts + KB vector search — добавляем инкрементально
3. **Dashboard** (Phase 3) = новый aggregator поверх Core + Extensions

Это позволяет:
- Выпустить Phase 1 за 2-3 дня с immediate value (configurable thresholds + ADI tracking)
- Phase 2 добавить в Sprint 12 или позже без breaking changes
- Тестировать Core без LanceDB (pure functions)

## Invariants

- **R_eff = min(evidence_scores)** — weakest link principle НИКОГДА не меняется на average. Это фундамент trust calculus.
- **Default config = текущее поведение** — пользователь без config.yaml получает те же результаты что сейчас. Zero breaking changes.
- **ADI output backward compatible** — старый `forgeplan reason` JSON формат продолжает работать. AdiRecord = надстройка, не замена.
- **Core не зависит от LanceDB** — trust.rs, adi.rs, model.rs = pure functions, testable без store.

## Rollback Plan

- Phase 1 код живёт в `fpf/core/` — если что-то пошло не так, старый `fpf/mod.rs` + `explore.rs` + `contexts.rs` остаются рабочими (не удаляются до Phase 3)
- FpfConfig с `Default::default()` = fallback на hardcoded values
- AdiRecord — отдельная таблица в LanceDB, не затрагивает существующие таблицы
- Git revert Phase 1 = один коммит, не ломает ничего

## Risks & Open Questions

- **Risk**: Rule engine YAML syntax может быть неудобен пользователям → mitigation: хорошие defaults + примеры
- **Risk**: FpfContext computation на каждый dashboard call может быть медленным для 1000+ артефактов → mitigation: lazy computation, cache with TTL
- **Open question**: AdiRecord как отдельный artifact kind или как поле в существующих артефактах?
- **Open question**: Нужен ли impact analysis в Phase 2 или это Sprint 12 (RFC-002 Graph Intelligence)?
- **Open question**: KB vector search — использовать существующий embedding pipeline или отдельный?

## Implementation Phases

### Phase 1: Core — Trust + ADI + Config (Sprint 11)
- [x] **1.1** `FpfConfig` struct + loading from config.yaml with defaults
- [x] **1.2** `core/trust.rs` — TrustScore computation (extract from fgr.rs + reff.rs, parameterize)
- [x] **1.3** `core/adi.rs` — AdiRecord/AdiSnapshot structs, serialize/deserialize, link to artifacts
- [x] **1.4** `core/model.rs` — FpfContext unified model (computed, not stored)
- [x] **1.5** Migrate `forgeplan reason --save` to create AdiRecord (structured JSON in Note body)
- [x] **1.6** Tests: 34 unit tests for core (target was ≥10)
- [x] **1.7** Wire FpfConfig into CLI (score, fgr, context, dashboard) + config templates in init

### Phase 2: Extensions — Rules + KB (Sprint 12)
- [ ] **2.1** `ext/rules.rs` — Declarative rule engine (YAML conditions → actions)
- [ ] **2.2** `ext/knowledge.rs` — KB vector search using LanceDB embeddings
- [ ] **2.3** `ext/contexts.rs` — Improved bounded context with semantic naming
- [ ] **2.4** Migrate explore.rs → rule engine with backward-compatible defaults
- [ ] **2.5** Tests: ≥8 unit tests for extensions

### Phase 3: Dashboard + Integration (Sprint 12)
- [ ] **3.1** `dashboard.rs` — New aggregator using FpfContext model
- [ ] **3.2** CLI output improvements (bounded context in reason, trust breakdown in score)
- [ ] **3.3** MCP tools update (fpf_context, fpf_rules)
- [ ] **3.4** Tests: integration tests with LanceDB store

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| EPIC-002 | Epic | child_of |
| PRD-002 | PRD | based_on |
| PRD-013 | PRD | refines |
| PROB-021 | Problem | addresses |
| NOTE-006 | Note | informed_by |

---

> **Next step**: ADI reasoning (`forgeplan reason RFC-001 --fpf`) → validate → discuss Options → ADR → implement Phase 1.

