---
depth: standard
id: RFC-005
kind: rfc
links:
- target: PRD-022
  relation: based_on
status: active
title: Estimate Engine — Multi-Grade Scoring Architecture
---

---
id: RFC-005
title: "Estimate Engine — Multi-Grade Scoring Architecture"
status: Draft
author: User + AI
created: 2026-03-31
updated: 2026-03-31
prd: PRD-022
depth: standard
---

# RFC-005: Estimate Engine — Multi-Grade Scoring Architecture

## Progress

```
Phase 1  ████████████████████████  5/5  (100%)
Phase 2  ████████████████████████  4/4  (100%)
Phase 3  ████████████████░░░░░░░░  2/3  ( 67%)
─────────────────────────────────────────────────
TOTAL                              11/12  ( 92%)
```

---

## Summary

Новый модуль `estimate` в forgeplan-core, который извлекает work items из артефактов (FR из PRD, Phases из RFC), назначает Fibonacci complexity, рассчитывает часы для 5 грейдов (Junior/Middle/Senior/PS/AI), и выводит таблицу с confidence scoring. CLI команда `forgeplan estimate <id>`.

## Motivation

Пользователь ведёт эстимейты вручную в Excel (148 задач, 24 спринта, 5 грейдов). Это disconnected от артефактов в Forgeplan. Задачи добавляются в PRD, но не попадают в план. AI-конверсия (human 3h → AI 5min) не формализована.

Forgeplan уже хранит FR в PRD и Phases в RFC — вся информация для эстимейтов есть, нужен только scoring engine.

Если НЕ делать: пользователь продолжает вести параллельную Excel-таблицу, которая расходится с артефактами.

## Goals

- Автоматический эстимейт из артефактов за <5s (без LLM) или <30s (с LLM)
- Multi-grade output: Junior/Middle/Senior/PS/AI в одной таблице
- AI-конверсия с task-type-aware multipliers (coding ×0.1, infra ×0.5)
- Confidence scoring по полноте артефакта
- Grade profile в config.yaml: домен→грейд

## Non-Goals

- Sprint planning / capacity planning (Phase 2 в PRD-022)
- Historical calibration из evidence (Phase 3 в PRD-022)
- Team-level aggregation (multiple people)
- Gantt / timeline visualization
- Cost/budget calculation

## Options Considered

### Option A: Rule-based Complexity (no LLM)

**Description**: Complexity определяется по эвристикам: длина описания FR, количество зависимостей, наличие ключевых слов ("integration", "migration", "design").

**Pros**: Мгновенно (<1s), детерминированно, работает offline, 0 cost

**Cons**: Неточно для нестандартных FR, не учитывает контекст кодовой базы

### Option B: LLM-powered Complexity (AI scoring)

**Description**: LLM анализирует каждый FR, определяет Fibonacci complexity и task-type (coding/infra/design). Prompt: "Given this FR and project context, assign Fibonacci complexity (1,2,3,5,8,13) and task type."

**Pros**: Учитывает семантику, ближе к human оценке, понимает контекст

**Cons**: 5-30s latency, стоит деньги, не детерминированно, требует API key

### Option C: Hybrid (rule-based + LLM refinement)

**Description**: Rule-based как fallback (default). LLM как opt-in refinement (`--ai-score`). При наличии API key — LLM автоматически, без — правила.

**Pros**: Работает всегда, LLM улучшает когда доступен, graceful degradation

**Cons**: Два кодовых пути, сложнее тестировать

## Trade-off Analysis

| Критерий | Option A: Rules | Option B: LLM | Option C: Hybrid |
|----------|----------------|---------------|-----------------|
| Complexity | Low | Medium | Medium+ |
| Latency | <1s | 5-30s | 1s / 30s |
| Cost | Free | Per-call | Free default |
| Accuracy | ~60% | ~80% | 60-80% |
| Offline support | Yes | No | Yes (degraded) |
| Determinism | Yes | No | Partial |
| Developer experience | Simple | Waiting | Best of both |

## Proposed Direction

**Option C: Hybrid** — rule-based по умолчанию, LLM opt-in.

Это следует паттерну Smart Routing (PRD-020): rule-based L0 всегда работает, LLM L1/L2 улучшает когда доступен. Тот же подход для estimate.

Дополнительно: пользователь может **вручную задать complexity** через `forgeplan update PRD-022 --set-complexity FR-001=5,FR-002=8` — override автоматики.

## Risks & Open Questions

- **R-1**: FR в PRD хранятся как markdown таблица — парсинг хрупкий. Mitigation: regex + fallback на LLM extraction.
- **R-2**: Fibonacci complexity субъективна — разные LLM дадут разные оценки. Mitigation: cache результат в metadata, allow manual override.
- **OQ-1**: Хранить ли complexity score в LanceDB как поле артефакта или отдельной таблицей? **Решение**: Отдельная таблица `estimates` — версионируется отдельно.
- **OQ-2**: Grade multipliers — configurable в config.yaml или hardcoded? **Решение**: Defaults hardcoded, overridable в config.yaml.

## Architecture

### New module: `crates/forgeplan-core/src/estimate/`

```
estimate/
├── mod.rs              — pub mod + re-exports
├── types.rs            — Grade, Complexity, EstimateItem, EstimateResult, GradeProfile, TaskType
├── extractor.rs        — extract_work_items(artifact) → Vec<WorkItem>
│                         Парсит FR таблицу из PRD, Phases из RFC
├── scorer.rs           — score_complexity(items, mode) → Vec<ScoredItem>
│                         Rule-based + LLM scorer
├── calculator.rs       — calculate_hours(scored_items, grade_config) → EstimateResult
│                         Fibonacci × grade_multiplier для каждого грейда
├── ai_converter.rs     — convert_to_ai(estimate, task_types) → AiEstimate
│                         Task-type-aware multipliers + review overhead
├── confidence.rs       — score_confidence(artifact, links) → f64
│                         Полнота: has FR (+30%), has RFC phases (+25%), has Spec (+15%)
└── display.rs          — format_table(result) → String
                          Terminal table с ANSI colors
```

### Core Types

```rust
#[derive(Debug, Clone, Copy)]
pub enum Grade {
    Junior,      // ×2.0
    Middle,      // ×1.5
    Senior,      // ×1.0 (baseline)
    Principal,   // ×0.7
    Ai,          // ×0.4 (conservative)
}

#[derive(Debug, Clone, Copy)]
pub enum Complexity {
    Trivial = 1,
    Simple = 2,
    Medium = 3,
    Complex = 5,
    Hard = 8,
    Epic = 13,
}

#[derive(Debug, Clone, Copy)]
pub enum TaskType {
    PureCoding,     // AI ×0.10
    CodingInfra,    // AI ×0.25
    DesignCoding,   // AI ×0.30
    PureInfra,      // AI ×0.50
    Coordination,   // AI ×1.00
}

pub struct GradeProfile {
    pub domains: HashMap<String, Grade>,  // "backend" → Middle
    pub default_grade: Grade,             // Senior
}

pub struct EstimateItem {
    pub id: String,           // "FR-001"
    pub description: String,
    pub complexity: Complexity,
    pub task_type: TaskType,
    pub hours: HashMap<Grade, (f64, f64)>,  // grade → (min_hours, max_hours)
}

pub struct EstimateResult {
    pub artifact_id: String,
    pub items: Vec<EstimateItem>,
    pub totals: HashMap<Grade, (f64, f64)>,
    pub confidence: f64,
    pub confidence_reasons: Vec<String>,
}
```

### Config extension

```yaml
# .forgeplan/config.yaml
estimate:
  grade_multipliers:
    junior: 2.0
    middle: 1.5
    senior: 1.0        # baseline
    principal: 0.7
    ai: 0.4
  ai_task_multipliers:
    pure_coding: 0.10
    coding_infra: 0.25
    design_coding: 0.30
    pure_infra: 0.50
    coordination: 1.00
  review_overhead: 0.30  # 30% of AI time for human review
  safety_margin: 0.50    # warn if sprint > 50% capacity
  grade_profile:
    backend: middle
    frontend: junior
    devops: senior
    ai_ml: principal
    default: senior
```

### CLI command

```
forgeplan estimate <artifact-id> [OPTIONS]

OPTIONS:
  --grade <GRADE>      Override grade for all items (junior|middle|senior|principal|ai)
  --my-grade           Use grade profile from config (domain-aware)
  --ai                 Show AI-converted hours with task-type multipliers
  --format <FMT>       Output: table (default), json, csv
  --no-llm             Force rule-based scoring only
```

### Output example

```
Estimate for PRD-022: AI Estimation Engine
Grade: Senior (baseline) | Confidence: 75%

  ID      Description                  Cmpl  Jun    Mid    Senior  PS     AI
  FR-001  Estimate command + breakdown   3   16h    12h      8h    5h     3h
  FR-002  FR/Phase extraction            5   26h    19h     13h    9h     5h
  FR-003  Fibonacci complexity scoring   2   10h     7h      5h    3h     2h
  FR-004  --grade flag                   1    6h     4h      3h    2h     2h
  FR-005  --ai conversion                3   16h    12h      8h    5h     3h
  FR-006  Grade profile config           2   10h     7h      5h    3h     2h
  FR-007  --my-grade auto-select         2   10h     7h      5h    3h     2h
  FR-008  Confidence scoring             2   10h     7h      5h    3h     2h
  ─────────────────────────────────────────────────────────────────────────
  TOTAL                                 20  104h    75h     52h   33h    21h
                                            13.0d   9.4d   6.5d  4.1d   2.6d

  Confidence: 75% — has FR (PRD-022), no Spec
  Boost: create RFC phases (+25%), add Spec (+15%)
```

## Implementation Phases

### Phase 1: Core Types + Rule-Based Scorer (MVP)
- [x] **1.1** Create `estimate/types.rs` — Grade, Complexity, TaskType, EstimateItem, EstimateResult, GradeProfile enums and structs
- [x] **1.2** Create `estimate/extractor.rs` — parse FR table from PRD markdown (regex-based), parse Phase checkboxes from RFC
- [x] **1.3** Create `estimate/scorer.rs` — rule-based complexity scoring (keyword heuristics + description length)
- [x] **1.4** Create `estimate/calculator.rs` — Fibonacci x grade_multiplier for all 5 grades, min/max range
- [x] **1.5** Create `estimate/display.rs` — terminal table formatting with aligned columns

### Phase 2: AI Conversion + Confidence + CLI
- [x] **2.1** Create `estimate/ai_converter.rs` — TaskType classification + AI multipliers + review overhead (integrated in calculator.rs)
- [x] **2.2** Create `estimate/confidence.rs` — artifact completeness scoring (FR +30%, RFC phases +25%, Spec +15%, Evidence +20%)
- [x] **2.3** Add `EstimateConfig` to config/types.rs — grade_multipliers, ai_task_multipliers, grade_profile, safety_margin (PR #74)
- [x] **2.4** Create `commands/estimate.rs` — CLI command with --grade, --my-grade, --llm-score, --complexity, --json flags

### Phase 3: LLM Scorer + MCP Tool
- [x] **3.1** Add LLM-based complexity scoring in scorer.rs — prompt engineering for Fibonacci assignment + task type classification (PR #79)
- [ ] **3.2** Add `estimate` MCP tool — same functionality as CLI, accessible via MCP server
- [x] **3.3** Add manual complexity override — `--complexity FR-001=5,FR-002=8` flag in CLI (commit 6920da9)

## Affected Files

### New files:
- `crates/forgeplan-core/src/estimate/mod.rs`
- `crates/forgeplan-core/src/estimate/types.rs`
- `crates/forgeplan-core/src/estimate/extractor.rs`
- `crates/forgeplan-core/src/estimate/scorer.rs`
- `crates/forgeplan-core/src/estimate/calculator.rs`
- `crates/forgeplan-core/src/estimate/ai_converter.rs`
- `crates/forgeplan-core/src/estimate/confidence.rs`
- `crates/forgeplan-core/src/estimate/display.rs`
- `crates/forgeplan-cli/src/commands/estimate.rs`

### Modified files:
- `crates/forgeplan-core/src/lib.rs` — add `pub mod estimate;`
- `crates/forgeplan-core/src/config/types.rs` — add `EstimateConfig`
- `crates/forgeplan-cli/src/commands/mod.rs` — register estimate command
- `crates/forgeplan-cli/src/main.rs` — add estimate subcommand

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PRD-022 | PRD | based_on |
| RFC-003 | RFC | extends (driver trait pattern) |
| PRD-020 | PRD | pattern_source (hybrid L0/L1 approach) |

---

> **Next step**: Implement Phase 1 -> test -> evidence -> review.

