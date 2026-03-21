# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Что это за проект

**Forgeplan** — универсальная Rust-платформа (CLI + Desktop App) для ведения любого проекта от идеи до реализации через структурированные артефакты с quality scoring, semantic search и evidence tracking.

**Формула**:
```
Forgeplan = Quint-code (decision engine, R_eff scoring, evidence decay)
          + BMAD (PRD workflow, 13-step validation, adversarial review)
          + OpenSpec (artifact DAG, delta-specs, custom schemas)
          + FPF (reasoning framework, ADI cycle, trust calculus)
          + git-adr (Rust CLI patterns, clap, templates)
          + LanceDB (embedded DB: tables + vectors в одном)
          + Tauri (desktop app: React UI + shared Rust core)
```

**CLI**: `forgeplan` (alias: `fpl`)
**Desktop**: Tauri 2.0 + React (shared Rust core)
**Язык документации**: русский. Код: Rust с английскими идентификаторами.

## Текущий статус

- **Phase 0** (Foundation & Research) — DONE
- **Phase 1** (Schemas & Templates) — IN PROGRESS
- **Phase 2** (Workflow & Integration) — не начат
- **Phase 3** (Rust CLI + LanceDB) — не начат, `src/` пуст
- **Phase 4** (Desktop App + AI) — не начат
- **Phase 5** (MCP Server) — не начат

Подробности: `PLAN.md` (49 задач, 5 фаз), `TODO.md` (текущие P0/P1/P2 приоритеты).

## Как начать работу в новом чате

1. **Прочитай `CONTEXT.md`** — полный контекст проекта (368 строк), содержит всё что нужно
2. **Для архитектуры** — `VISION.md` (560 строк): data model, tech stack, modules, все 17 секций
3. **Для текущих задач** — `TODO.md` → `PLAN.md`
4. **Для gap analysis** — `COMPLETENESS-CHECK.md` (52 компонента, 10 слоёв)
5. **Для поиска инструментов** — `SOURCES.md` (карта skills, commands, agents, repos)
6. **Для reference code** — `sources/` (read-only repos, см. таблицу ниже)
7. **Используй Hindsight** — `memory_recall("Forgeplan")` для быстрого восстановления контекста

## Структура проекта

```
ForgePlan/
├── CONTEXT.md              ← НАЧНИ ЗДЕСЬ — полный контекст для нового чата
├── VISION.md               ← Архитектура: data model, tech stack, screens, phases
├── PLAN.md                 ← 49 задач, 5 фаз с progress bars
├── TODO.md                 ← Текущие приоритеты P0/P1/P2
├── COMPLETENESS-CHECK.md   ← Gap analysis: 52 компонента, 10 слоёв
├── SOURCES.md              ← Карта всех источников
│
├── docs/
│   ├── schemas/            ← Формальные правила артефактов
│   │   ├── PRD-SCHEMA.md   ← Обязательные секции PRD, depth calibration, validation
│   │   ├── EPIC-SCHEMA.md  ← Aggregated progress, children rules
│   │   └── SPEC-SCHEMA.md  ← API contracts, data models, versioning
│   ├── guides/
│   │   ├── ARTIFACT-MODEL.md    ← Иерархия: Epic→PRD→Spec→RFC→ADR + lifecycle
│   │   └── PRD-RFC-ADR-FLOW.md  ← Decision tree: какой документ создать
│   ├── references/
│   │   ├── REF-DOCS-ANALYSIS.md ← Анализ 10 методологий
│   │   └── SKILLS-AUDIT.md      ← 52 skills по 10 слоям + gaps
│   └── ref/                ← Raw reference docs (Word, Markdown) на русском
│
├── templates/              ← Markdown шаблоны (_TEMPLATE.md)
│   ├── prd/                ← PRD шаблон
│   ├── epic/               ← Epic шаблон
│   ├── spec/               ← Specification шаблон
│   ├── rfc/                ← RFC шаблон
│   └── adr/                ← ADR шаблон
│
├── sources/                ← Reference implementations (READ-ONLY, не редактировать!)
│   ├── quint-code/         ← Go — data model, R_eff scoring, SQLite schema
│   ├── git-adr/            ← Rust — CLI patterns (clap), templates
│   ├── OpenSpec/           ← TypeScript — artifact DAG, delta-specs
│   ├── BMAD-METHOD/        ← Markdown — PRD workflow, 13 validation steps
│   ├── adr-tools/          ← Bash — original ADR CLI
│   └── ccpm/               ← Markdown — Claude Code project management
│
├── research/               ← Исследования методологий
└── src/                    ← Rust CLI (пусто до Phase 3)
```

## Артефакты (11 типов)

### Из Quint-code (6):
| Kind | Prefix | Описание |
|------|--------|----------|
| Note | `note-` | Микро-решение |
| ProblemCard | `prob-` | Проблема с контекстом |
| SolutionPortfolio | `sol-` | 2-3+ варианта (weakest link scoring) |
| DecisionRecord | `dec-` | DDR: invariants + rollback + valid_until |
| EvidencePack | `evid-` | Тесты, benchmarks, measurements |
| RefreshReport | `ref-` | Переоценка stale решений |

### Новые для Forgeplan (5):
| Kind | Prefix | Описание |
|------|--------|----------|
| PRD | `prd-` | Product Requirements Document |
| Epic | `epic-` | Группирует PRD[], RFC[], ADR[] |
| Spec | `spec-` | API contracts, data models |
| RFC | `rfc-` | Архитектурное предложение с фазами |
| ADR | `adr-` | Architecture Decision Record |

### Иерархия
```
Epic (стратегия) → PRD[] (что и зачем) → Spec[] (контракты) + RFC[] (как строим) + ADR[] (почему так)
```

### Lifecycle flow
```
Small task  → RFC only
Medium task → PRD → RFC → Sprint
Large task  → Epic → PRD[] → Spec[] → RFC[] → ADR[] → Sprint[]
```

## Ключевые формулы и паттерны

### R_eff scoring (из Quint-code)
```
R_eff = min(evidence_scores) — trust = weakest link, НИКОГДА average
```
- Evidence Decay: `valid_until` TTL, expired evidence = 0.1 (stale, not absent)
- CL penalty: CL3=0.0, CL2=0.1, CL1=0.4, CL0=0.9
- DerivedStatus: UNDERFRAMED → FRAMED → EXPLORING → COMPARED → DECIDED → APPLIED

### Depth Calibration
| Complexity | Depth | Создаём |
|-----------|-------|---------|
| Quick fix, 1 файл | Tactical | Note или ничего |
| Фича 1-3 дня | Standard | PRD (tactical) → RFC |
| Новый модуль, 1-2 нед | Deep | PRD → Spec → RFC → ADR |
| Подсистема, кросс-команда | Critical | Epic → PRD[] → Spec[] → RFC[] → ADR[] |

### Workflow паттерны
- **Adversarial Review** (BMAD) — reviewer MUST find problems; 0 issues = re-review
- **Delta-specs** (OpenSpec) — describe ONLY changes: ADDED/MODIFIED/REMOVED
- **ADI cycle** (FPF) — Abduction (3+ hypotheses) → Deduction → Induction
- **Pipeline = guideline**, NOT rigid sequence (подтверждено FPF автором)
- **Contextual chain** — output каждой фазы = input следующей

## Storage: LanceDB + Markdown (dual)

- **LanceDB** = source of truth (structured tables + vector embeddings)
- **Markdown** = human-readable projections (git-tracked)
- Sync on every write: write to LanceDB → render markdown → save to `.forgeplan/`

```
.forgeplan/          ← создаётся forgeplan init в целевом проекте
├── config.yaml
├── lance/           ← LanceDB (gitignore)
├── prds/            ← markdown (git-tracked)
├── epics/, specs/, rfcs/, adrs/
├── problems/, solutions/, decisions/
├── evidence/, notes/, refresh/
```

## Planned Rust Architecture (Phase 3+)

```
forgeplan/
├── Cargo.toml                    ← workspace root
├── crates/
│   ├── forgeplan-core/           ← SHARED LIBRARY (вся логика)
│   │   ├── artifact/             ← types, parser, writer, store
│   │   ├── scoring/              ← R_eff = min(evidence_scores)
│   │   ├── validation/           ← BMAD 13-step rules
│   │   ├── search/               ← LanceDB vectors + Tantivy text
│   │   ├── progress/             ← checkbox parser + progress bars
│   │   ├── graph/                ← mermaid dependency graph
│   │   ├── embed/                ← ONNX local embeddings (BGE-M3)
│   │   ├── db/                   ← LanceDB operations
│   │   ├── template/             ← tera engine
│   │   └── config/               ← .forgeplan/config.yaml
│   ├── forgeplan-cli/            ← CLI binary (clap derive)
│   ├── forgeplan-tauri/          ← Desktop app backend (Tauri 2.0 + core)
│   └── forgeplan-mcp/            ← MCP server (Phase 5, rmcp)
├── apps/desktop/                 ← React frontend (Tauri UI)
└── templates/                    ← .md.tera шаблоны (embedded in binary)
```

Key dependencies: `clap` (derive), `lancedb`, `tantivy`, `tera`, `pulldown-cmark`, `serde_yaml`, `ort` (ONNX Runtime), `rmcp`.

## Reference Code — что откуда портировать

| Что портируем | Откуда | Куда (Rust) |
|--------------|--------|-------------|
| Data model (ArtifactKind, Meta, Link) | `sources/quint-code/src/mcp/internal/artifact/types.go` | `crates/forgeplan-core/src/artifact/types.rs` |
| R_eff scoring (52 LOC) | `sources/quint-code/src/mcp/internal/reff/reff.go` | `crates/forgeplan-core/src/scoring/reff.rs` |
| SQLite schema (9 tables) | `sources/quint-code/src/mcp/schema.sql` | Адаптация под LanceDB tables |
| CLI patterns (clap) | `sources/git-adr/src/cli/` | `crates/forgeplan-cli/src/commands/` |
| Template engine | `sources/git-adr/src/core/templates.rs` | `crates/forgeplan-core/src/template/` |
| PRD validation (13 steps) | `sources/BMAD-METHOD/src/bmm-skills/2-plan-workflows/create-prd/` | `crates/forgeplan-core/src/validation/` |
| Artifact DAG, delta-specs | `sources/OpenSpec/src/core/` | `crates/forgeplan-core/src/artifact/` |
| Slash commands UX | `sources/quint-code/src/mcp/cmd/commands/*.md` | CLI UX design |

## Non-Goals

- НЕ project management (не Jira/Linear)
- НЕ CI/CD, НЕ SaaS, НЕ code generator
- Local-first, single binary, git для sync
