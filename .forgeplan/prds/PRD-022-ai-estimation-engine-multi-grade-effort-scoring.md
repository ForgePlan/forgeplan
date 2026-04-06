---
depth: standard
id: PRD-022
kind: prd
links:
- target: EPIC-001
  relation: refines
- target: PRD-020
  relation: based_on
- target: RFC-003
  relation: based_on
status: active
title: AI Estimation Engine — Multi-Grade Effort Scoring
---

---
id: PRD-022
title: "AI Estimation Engine — Multi-Grade Effort Scoring"
status: Draft
author: User + AI
created: 2026-03-31
updated: 2026-03-31
epic: EPIC-001
priority: P1
depth: standard
domain: general
projectType: cli_tool
stepsCompleted: []
---

# PRD-022: AI Estimation Engine — Multi-Grade Effort Scoring

## Progress

```
Phase 1  █████████████████████░░░  7/8  ( 88%)
─────────────────────────────────────────────────
TOTAL                               7/8  ( 88%)
```

---

## Executive Summary

### Vision

Forgeplan автоматически рассчитывает эстимейты трудозатрат из артефактов (PRD/RFC/Spec), учитывая грейд исполнителя, тип задачи и AI-конверсию — превращая документацию в реалистичный план с capacity planning.

### Problem

Сейчас оценка трудозатрат делается вручную в Excel-таблицах: пользователь разбивает задачи, назначает Fibonacci complexity, считает часы для каждого грейда, планирует спринты. Это занимает часы и disconnected от артефактов в Forgeplan.

При этом AI-агенты выполняют задачи в 10-40x быстрее humans (задача на 3h senior = 5-10 минут AI), но нет инструмента для конверсии human→AI эстимейтов и планирования AI-спринтов.

**Impact**: Пользователь тратит 2-4 часа на ручное планирование каждого спринта. Disconnect между PRD (что делать) и capacity plan (когда и кем) создаёт рассинхрон — задачи добавляются в PRD, но не попадают в план.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| Solo Developer | Инженер-мульти-стек с разными грейдами по направлениям | Не знает свой реальный capacity: senior в DevOps, junior во frontend — спринты или перегружены, или пусты |
| AI-first Developer | Разработчик, делегирующий кодинг AI-агентам | Human-эстимейты (3h) бесполезны — AI делает за 10 минут, но нет инструмента конверсии |
| Tech Lead | Планирует работу команды с разными грейдами | Ручной пересчёт для каждого члена команды; нет единого источника правды |

### Differentiators

- **Артефакт-driven**: эстимейты берутся из FR в PRD и Phases в RFC, а не из отдельной системы
- **Multi-grade profile**: один человек = разные грейды в разных доменах (не "one size fits all")
- **AI-конверсия**: автоматический пересчёт human→AI с учётом типа задачи (coding vs infra vs design)
- **Capacity planning с safety margin**: загрузка спринта до 40-50% — встроенный буфер на ошибки
- **Evidence-calibrated**: реальные данные из прошлых задач корректируют будущие эстимейты

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | Estimate generation time | Seconds per PRD | Manual: 30-60min | < 10s automated | v1.0 | CLI timing |
| SC-2 | Estimate accuracy (human) | Actual/Estimated ratio | N/A (no baseline) | 0.7-1.3 range for 80% tasks | After 20 tasks | Evidence comparison |
| SC-3 | AI conversion accuracy | AI actual/estimated ratio | N/A | 0.5-2.0 range for 70% tasks | After 10 AI tasks | Evidence comparison |
| SC-4 | Grade profile coverage | Domains with grade assignment | 0 | 5+ domains configurable | v1.0 | Config check |

---

## Product Scope

### MVP (In-Scope)

- `forgeplan estimate <artifact-id>` — рассчитать эстимейт из FR/Phases
- `forgeplan estimate <artifact-id> --grade middle` — пересчитать для конкретного грейда
- `forgeplan estimate <artifact-id> --ai` — конверсия в AI-часы
- Grade profile в config.yaml: маппинг домен→грейд для пользователя
- Fibonacci complexity scoring (1,2,3,5,8,13)
- Grade multipliers: Junior×2.0, Middle×1.5, Senior×1.0 (baseline), Principal×0.7, AI×0.4
- AI task-type multipliers: coding×0.1, coding+infra×0.25, design+coding×0.3, infra×0.5
- Confidence scoring: зависит от полноты артефакта (есть FR? есть RFC phases? есть Spec?)
- Output: таблица FR→hours per grade + total + confidence

### Out of Scope

- Sprint planning / capacity planning (Phase 2)
- Historical calibration from evidence (Phase 3)
- Gantt chart / timeline visualization
- Team-level aggregation (multiple people)
- Integration with external tools (Jira, Linear)
- Cost/budget calculation (рубли/доллары)

### Growth Vision

- Phase 2: `forgeplan sprint plan` — автоматическая раскладка задач по спринтам с capacity check
- Phase 3: Evidence-driven calibration — реальные vs оценённые часы корректируют multipliers
- Phase 4: Team profiles — несколько человек, агрегированный capacity

---

## User Journeys

### Journey 1: Solo Developer — Оценка задачи перед началом работы

**Цель пользователя**: Понять сколько займёт задача, учитывая свой грейд в этом домене.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan estimate PRD-018` | Парсит FR из PRD, определяет complexity каждого FR | AI-powered extraction |
| 2 | Видит таблицу: FR→complexity→hours per grade | Показывает Junior/Middle/Senior/PS/AI столбцы | Как в Excel таблице |
| 3 | Видит confidence score | 65% (no RFC phases) | Подсказка: "create RFC for better estimate" |
| 4 | `forgeplan estimate PRD-018 --grade middle` | Пересчёт с Middle multiplier для всех FR | Для Backend-задач |

**Результат**: Разработчик знает что PRD-018 займёт 13-21h (Middle) и может планировать спринт.

### Journey 2: AI-first Developer — Конверсия в AI-время

**Цель пользователя**: Понять сколько AI-агенты потратят на задачу из PRD.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan estimate PRD-018 --ai` | Классифицирует каждый FR по task-type (coding/infra/design) | AI определяет тип |
| 2 | Видит AI-столбец с дифференцированными multipliers | FR-001 (coding): 0.3h, FR-002 (coding): 0.5h, FR-003 (infra): 2h | Разные multipliers |
| 3 | Видит total AI time + human review overhead | AI: 2.8h + review: 1h = 3.8h total | Review = 30% of AI time |

**Результат**: Разработчик знает что AI сделает PRD-018 за ~4h (включая review), а не 21h Middle.

### Journey 3: Developer — Настройка grade profile

**Цель пользователя**: Указать свой грейд по каждому направлению для точных эстимейтов.

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan config set grade.devops senior` | Сохраняет в config.yaml | Или через редактирование yaml |
| 2 | `forgeplan config set grade.backend middle` | Обновляет profile | |
| 3 | `forgeplan config set grade.frontend junior` | Обновляет profile | |
| 4 | `forgeplan estimate PRD-018 --my-grade` | Определяет домен PRD, выбирает нужный grade, считает | Автоматический выбор |

**Результат**: `--my-grade` использует correct multiplier на основе домена задачи.

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | User can run `estimate` on any artifact and receive effort breakdown by grade (Junior/Middle/Senior/PS/AI) | Journey 1 |
| FR-002 | Core | Must | System can extract work items from FR table in PRD and Implementation Phases in RFC | Journey 1 |
| FR-003 | Core | Must | System can assign Fibonacci complexity (1,2,3,5,8,13) to each work item | Journey 1 |
| FR-004 | Core | Must | User can specify target grade via `--grade` flag to see hours for that grade | Journey 1 |
| FR-005 | AI | Must | User can run `estimate --ai` to see AI-converted hours with task-type-aware multipliers | Journey 2 |
| FR-006 | Config | Should | User can configure per-domain grade profile in config.yaml | Journey 3 |
| FR-007 | Config | Should | User can run `estimate --my-grade` to auto-select grade based on artifact domain | Journey 3 |
| FR-008 | UX | Should | System displays confidence score based on artifact completeness (has FR, has RFC phases, has Spec) | Journey 1 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | Estimate generation shall complete | < 5s without LLM, < 30s with LLM | Single artifact | CLI timing |
| NFR-002 | Accuracy | Grade multipliers shall be configurable | User-editable config | config.yaml | Config validation |
| NFR-003 | UX | Output shall be readable in terminal | Aligned columns, ANSI colors | 80-char terminal | Visual check |

---

## Acceptance Criteria

### AC-1: Basic Estimate

```gherkin
Given a PRD with 3 FR entries
When  user runs `forgeplan estimate PRD-022`
Then  system displays table with 3 rows (one per FR)
And   each row shows: FR-ID, description, complexity, Junior/Middle/Senior/PS/AI hours
And   bottom shows total and confidence score
```

### AC-2: Grade Override

```gherkin
Given a PRD with FR entries
When  user runs `forgeplan estimate PRD-022 --grade junior`
Then  all hours are calculated using Junior multiplier (×2.0)
And   header shows "Grade: Junior Developer"
```

### AC-3: AI Conversion

```gherkin
Given a PRD with FR entries of mixed types (coding, infra, design)
When  user runs `forgeplan estimate PRD-022 --ai`
Then  each FR shows AI hours calculated with task-type-specific multiplier
And   total includes human review overhead (30% of AI time)
```

---

## Dependencies

| Dependency | Type | Status | Owner |
|-----------|------|--------|-------|
| LLM integration (forgeplan-core/llm) | Technical | Ready | Core |
| Artifact store (forgeplan-core/db) | Technical | Ready | Core |
| Validation engine (forgeplan-core/validation) | Technical | Ready | Core |
| FR parser (frontmatter + markdown tables) | Technical | Needs extension | Core |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | AI complexity scoring inconsistent across runs | High | Medium | Cache complexity scores in artifact metadata; allow manual override | Core |
| R-2 | Grade multipliers (×2.0, ×1.5, etc.) don't match real-world ratios | Medium | High | Make multipliers configurable in config.yaml; calibrate from evidence over time | Config |
| R-3 | FR extraction fails on non-standard PRD formats | Medium | Medium | Fallback to whole-artifact estimation when table parsing fails | Core |
| R-4 | Users trust AI estimates blindly without safety margin | Low | High | Always show confidence %; add warning when confidence < 50% | UX |

---

## Affected Files

- `crates/forgeplan-core/src/estimate/` — NEW module
- `crates/forgeplan-core/src/estimate/types.rs` — EstimateResult, GradeProfile, Complexity
- `crates/forgeplan-core/src/estimate/scorer.rs` — Fibonacci scoring, grade multipliers
- `crates/forgeplan-core/src/estimate/extractor.rs` — FR/Phase extraction from artifacts
- `crates/forgeplan-core/src/estimate/ai_converter.rs` — Human→AI conversion
- `crates/forgeplan-core/src/estimate/confidence.rs` — Confidence scoring
- `crates/forgeplan-cli/src/commands/estimate.rs` — CLI command
- `crates/forgeplan-core/src/config/` — Grade profile in config.yaml

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-001 | Parent epic | active |
| PRD-020 | LLM Smart Routing (shares LLM infra) | active |
| RFC-003 | Layered Architecture (driver traits) | active |

---

> **Next step**: После approve -> создать RFC с Implementation Phases для архитектуры estimate module.






