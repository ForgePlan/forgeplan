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

- **v0.7.0** released — EPIC-001 complete
- **33 CLI команд**, **26 MCP tools**, **225 тестов**
- **20 dogfood артефактов** в LanceDB (5 active, 15 draft)
- **Phase 0–4** — DONE
- **Phase 5** (Desktop App, Tauri) — backlog

Подробности: `TODO.md` (текущие приоритеты).

## Как начать работу в новом чате

1. **Прочитай этот файл** — CLAUDE.md содержит CLI workflow, методологию, git-конвенции
2. **`forgeplan health`** — понять текущее состояние проекта (artifacts, blind spots, next actions)
3. **Для текущих задач** — `TODO.md`
4. **Полный гайд по CLI и методологии** — `docs/guides/FORGEPLAN-GUIDE.md`
5. **Для reference code** — `sources/` (read-only repos, см. таблицу ниже)
6. **Используй Hindsight** — `memory_recall("Forgeplan")` для быстрого восстановления контекста

### ОБЯЗАТЕЛЬНО перед работой над задачей:

```bash
forgeplan route "описание задачи"   # определи depth и pipeline
```

Если route говорит Standard+ → создай артефакт ПЕРЕД кодингом. Если Tactical → просто делай.

## Как пользоваться Forgeplan CLI (MCP-first)

> Forgeplan — MCP-first tool. Основной потребитель = AI агент через MCP server.
> CLI = secondary interface для human inspection.

### Core workflow (6 шагов):

```bash
# 1. Session start — понять состояние проекта
forgeplan health

# 2. Перед работой — определить depth и pipeline
forgeplan route "описание задачи"
# → Depth: Standard, Pipeline: PRD → RFC, Confidence: 85%

# 3. Создать артефакт
forgeplan new prd "Auth System"

# 4. Проверить качество
forgeplan validate PRD-001
# → MUST: Missing Problem section
# → SHOULD: density < 50 words

# 5. Когда готов — review и activate
forgeplan review PRD-001
# → Review PASSED — ready to activate
forgeplan activate PRD-001
# → draft → active

# 6. Подтвердить решение evidence
forgeplan new evidence "Benchmark results for auth approach"
forgeplan link EVID-001 PRD-001 --relation informs
forgeplan score PRD-001
# → R_eff = 1.00 (was 0.00)
```

### EvidencePack — как создавать (ВАЖНО):

EvidencePack ОБЯЗАТЕЛЬНО должен содержать structured fields в body для корректного R_eff scoring:

```markdown
## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement
```

| Field | Значения | Описание |
|-------|----------|----------|
| `verdict` | supports / weakens / refutes | Подтверждает, ослабляет или опровергает решение |
| `congruence_level` | 0-3 | CL3=same context (best), CL0=opposed context (worst) |
| `evidence_type` | measurement / test / benchmark / audit | Тип доказательства |

Без structured fields R_eff parser не найдёт данные и выставит CL0 (penalty 0.9).

### Lifecycle commands:

```bash
forgeplan review <id>              # проверить готовность
forgeplan activate <id>            # draft → active (validation gate)
forgeplan supersede <id> --by <new> # active → superseded + chain warnings
forgeplan deprecate <id> --reason "..." # active → deprecated
```

Notes и Problems не требуют validation gate для activation.
PRD, RFC, ADR, Epic, Spec — MUST rules должны пройти.

### Validator aliases:

Validator принимает синонимы для секций:
- `## Problem` = `## Motivation` = `## Problem Statement` = `## Background`
- `## Goals` = `## Success Criteria` = `## Objectives`
- `## Non-Goals` = `## Out of Scope` = `## Product Scope`
- `## Related` = `## Related Artifacts` = `## Dependencies`
- `## Target Users` = `## Target Audience` = `## Users`

### Dogfood insights (из реального использования):

1. **Создавай артефакт → сразу заполняй MUST секции** — иначе review fail
2. **Evidence делает R_eff живым** — без evidence все scores = 0.0, health кричит "At Risk"
3. **Не создавай все 10 типов** — реально используются 6: PRD, RFC, ADR, Note, Problem, Epic
4. **route перед работой** — определяет depth и pipeline, экономит время
5. **health на session start** — показывает orphans, blind spots, at risk

## Как пользоваться методологией (quick reference)

> Полный гайд: `docs/guides/HOW-TO-USE.md`

### Routing — один вопрос определяет depth:
```
Тривиально, обратимо за день?  → Tactical: ничего или Note
Фича 1-3 дня, есть выбор?      → Standard: Brief/PRD → RFC
Необратимо, 1-2 недели?        → Deep: PRD → Spec → RFC → ADR
Кросс-команда, стратегия?       → Critical: Epic → PRD[] → Spec[] → RFC[] → ADR[]
```

### 5 артефактов = 5 вопросов:
| Вопрос | Артефакт | Когда НЕ нужен |
|--------|----------|----------------|
| ЧТО и зачем? | PRD / Brief | Баг-фикс, рефакторинг |
| КАК ТОЧНО работает? | Spec | Нет API / data model changes |
| КАК СТРОИМ? | RFC | Архитектура очевидна, <1 дня |
| ПОЧЕМУ именно это? | ADR | Решение тривиально и обратимо |
| ГРУППИРОВКА? | Epic | Задача = один PRD |

### Правила:
- **Pipeline = guideline, НЕ бюрократия** — не создавай все 10 типов на каждую задачу
- **[Actor] can [capability]** — формат FR, без технологий в требованиях
- **Ребёнок ссылается на родителя** — PRD→Epic, RFC→PRD, ADR→RFC
- **Supersede, не удаляй** — старый артефакт получает status: Superseded
- **Quality gates по depth** — tactical: ничего, standard: Verification Gate, deep+: Adversarial Review

### Progress Tracking (ОБЯЗАТЕЛЬНО):
После завершения блока работ (реализация FR, закрытие фазы, создание артефакта) — **предложи пользователю обновить прогресс** в следующих местах:
1. **RFC** — чекбоксы Implementation Phases (`- [ ]` → `- [x]`) + progress bar
2. **PRD** — progress bar по FR (сколько FR реализовано)
3. **Epic** — Children таблица (progress %), aggregated progress bar
4. **PLAN.md** — Phase progress bar + чекбоксы задач
5. **TODO.md** — переместить завершённые задачи в Done ✅, обновить P0

Формула: **работа не закончена, пока прогресс не отражён в артефактах.**

### Git-конвенции

#### Формат коммита (Conventional Commits + Forgeplan):
```
<type>(<scope>): <description>

[body — что и почему, на русском]

Refs: RFC-001, FR-001..004
```

#### Types:
| Type | Когда | Пример |
|------|-------|--------|
| `feat` | Новая функциональность (FR-*) | `feat(cli): implement forgeplan init` |
| `docs` | Артефакты методологии (RFC, PRD, ADR) | `docs(rfc): add RFC-001 CLI architecture` |
| `fix` | Баг-фикс | `fix(frontmatter): handle missing closing ---` |
| `refactor` | Рефакторинг без изменения поведения | `refactor(store): extract slugify` |
| `test` | Тесты | `test(workspace): add init roundtrip tests` |
| `chore` | Build, deps, CI | `chore(deps): add tempfile dev-dependency` |
| `progress` | Обновление прогресса артефактов | `progress: update Phase 3A tracking` |

#### Scope = модуль или артефакт:
- Код: `cli`, `core`, `store`, `template`, `scoring`, `workspace`, `config`
- Артефакты: `rfc`, `prd`, `adr`, `epic`

#### Branching Strategy (Trunk-based):
```
main                              ← primary branch (tagged releases: v0.7.0)
  ├── feat/epic-001-completion    ← feature branch
  ├── fix/dogfood-findings        ← bugfix branch
  └── docs/rfc-002-lancedb       ← docs-only branch
```

| Ветка | Мерджится в | Стратегия |
|-------|-------------|-----------|
| `feat/*`, `fix/*`, `docs/*` | `main` | Squash merge via PR |

Формат имени: `{type}/{slug}` — `feat/epic-001-completion`, `fix/dogfood-findings`

#### Lifecycle ветки:
```
1. git checkout main && git checkout -b feat/my-feature
2. ... работа, коммиты ...
3. git push origin feat/my-feature
4. gh pr create → squash merge в main (НЕ удалять ветку)
5. git checkout main && git pull
6. При release: git tag -a v0.x.0 && git push origin v0.x.0
```

#### Правила коммитов:
- **Refs обязательны** — каждый коммит ссылается на артефакт (RFC, FR, ADR)
- **Один коммит = одна логическая единица** — не мешать feat + docs + refactor
- **Description на английском** (для совместимости), body на русском (для контекста)
- **Не коммить напрямую в main или dev** — всегда через feature branch + PR

#### PR и merge:
- **PR title** = `[ARTIFACT-ID] description` — `[RFC-001] Implement Phase 3A core CLI`
- **PR body** = Summary (bullets) + Refs (артефакты) + Test plan
- **feat/* → dev**: Squash merge (чистая история)
- **release/* → main**: Merge commit (сохраняет историю RC)
- **НЕ удалять ветки после merge** — feature и release branches сохраняются как история
- **После merge в main**: tag + sync dev from main

#### Релизы:
- **Формат тега**: `v{major}.{minor}.{patch}` — `v0.1.0`, `v0.2.0`
- **Когда**: после завершения Phase (3A → v0.1.0, 3B → v0.2.0, 3C → v1.0.0)
- **Процесс**: `dev` → `release/v0.x.0` (RC) → тесты → фиксы → `main` + tag
- **Release notes**: автогенерация из conventional commits
- **Binary**: `cargo build --release` (152MB с LanceDB+Arrow+tokio)

#### Worktrees (параллельная работа):
```bash
# Создать worktree для параллельной задачи (hotfix во время фичи)
git worktree add ../forgeplan-fix fix/frontmatter-parser

# Вернуться и удалить после merge
git worktree remove ../forgeplan-fix
```
- **Когда**: hotfix во время долгой фичи; параллельная работа агентов (isolation: "worktree")
- **Правило**: worktree = временный, удалять после merge

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
│   │   ├── FORGEPLAN-GUIDE.md   ← **ПОЛНЫЙ ГАЙД** — методология + CLI + evidence + lifecycle
│   │   ├── HOW-TO-USE.md        ← 10 правил методологии с примерами
│   │   ├── ARTIFACT-MODEL.md    ← Иерархия: Epic→PRD→Spec→RFC→ADR + lifecycle
│   │   ├── PRD-RFC-ADR-FLOW.md  ← Decision tree: какой документ создать
│   │   ├── DEPTH-CALIBRATION.md ← Tactical→Standard→Deep→Critical + auto-escalation
│   │   ├── QUALITY-GATES.md     ← Verification Gate + Adversarial Review + R_eff
│   │   └── GLOSSARY.md          ← 31 термин + lifecycle таблица
│   ├── epics/              ← Dogfood: EPIC-001-build-forgeplan.md
│   ├── prds/               ← Dogfood: PRD-001-forgeplan-cli.md
│   ├── adrs/               ← Dogfood: ADR-001..003 (Rust, LanceDB, DEC→ADR merge)
│   ├── references/
│   │   ├── REF-DOCS-ANALYSIS.md ← Анализ 10 методологий
│   │   └── SKILLS-AUDIT.md      ← 52 skills по 10 слоям + gaps
│   └── ref/                ← Raw reference docs (Word, Markdown) на русском
│
├── templates/              ← Markdown шаблоны (_TEMPLATE.md) — все с YAML frontmatter
│   ├── prd/                ← PRD (обогащён BMAD 13-step validation)
│   ├── brief/              ← Product Brief (lightweight tactical PRD)
│   ├── epic/               ← Epic
│   ├── spec/               ← Specification
│   ├── rfc/                ← RFC (с Implementation Phases)
│   ├── adr/                ← ADR (на deep+ включает DDR: invariants, rollback)
│   ├── problem/            ← ProblemCard (signal, Anti-Goodhart indicators)
│   ├── solution/           ← SolutionPortfolio (variants, weakest link)
│   ├── note/               ← Note (auto-expires 90 days)
│   ├── evidence/           ← EvidencePack (verdict, CL, valid_until → R_eff)
│   └── refresh/            ← RefreshReport (re-evaluation of stale artifacts)
│
├── sources/                ← Reference implementations (READ-ONLY, не редактировать!)
│   ├── quint-code/         ← Go — data model, R_eff scoring, SQLite schema
│   ├── git-adr/            ← Rust — CLI patterns (clap), templates
│   ├── OpenSpec/           ← TypeScript — artifact DAG, delta-specs
│   ├── BMAD-METHOD/        ← Markdown — PRD workflow, 13 validation steps
│   ├── adr-tools/          ← Bash — original ADR CLI
│   └── ccpm/               ← Markdown — Claude Code project management
│
├── crates/                 ← Rust workspace (core + cli + mcp)
└── research/               ← Исследования методологий
```

## Артефакты (10 типов)

### Из Quint-code (5):
| Kind | Prefix | Описание |
|------|--------|----------|
| Note | `note-` | Микро-решение |
| ProblemCard | `prob-` | Проблема с контекстом |
| SolutionPortfolio | `sol-` | 2-3+ варианта (weakest link scoring) |
| EvidencePack | `evid-` | Тесты, benchmarks, measurements |
| RefreshReport | `ref-` | Переоценка stale решений |

### Новые для Forgeplan (5):
| Kind | Prefix | Описание |
|------|--------|----------|
| PRD | `prd-` | Product Requirements Document |
| Epic | `epic-` | Группирует PRD[], RFC[], ADR[] |
| Spec | `spec-` | API contracts, data models |
| RFC | `rfc-` | Архитектурное предложение с фазами |
| ADR | `adr-` | Architecture Decision Record (на deep+ включает DDR-поля: invariants, rollback, valid_until) |

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

## Storage: LanceDB primary

- **LanceDB** = sole source of truth (structured tables + vector embeddings)
- **Markdown** = projections generated at `forgeplan new` (git-tracked, read-only after creation)
- Mutations через `forgeplan update` обновляют только LanceDB, не markdown

```
.forgeplan/          ← создаётся forgeplan init в целевом проекте
├── config.yaml
├── lance/           ← LanceDB (gitignore)
├── prds/            ← markdown (git-tracked)
├── epics/, specs/, rfcs/, adrs/
├── problems/, solutions/
├── evidence/, notes/, refresh/
```

## Rust Architecture (реализовано)

```
crates/
├── forgeplan-core/               ← SHARED LIBRARY (12.8K LOC, 194 теста)
│   ├── artifact/                 ← types, frontmatter parser
│   ├── config/                   ← .forgeplan/config.yaml
│   ├── db/                       ← LanceDB store (CRUD, relations, search)
│   ├── depth/                    ← depth calibration heuristics
│   ├── embed/                    ← fastembed (BGE-M3, behind feature flag)
│   ├── fpf/                      ← FPF engine: bounded contexts, explore-exploit
│   ├── graph/                    ← mermaid dependency graph
│   ├── health/                   ← project health dashboard
│   ├── journal/                  ← decision journal with R_eff
│   ├── lifecycle/                ← review → activate → supersede/deprecate
│   ├── link/                     ← typed artifact relationships
│   ├── llm/                      ← LLM integration (generate, reason, route, capture)
│   ├── progress/                 ← checkbox parser + ASCII progress bars
│   ├── projection/               ← markdown projection (LanceDB → .md)
│   ├── routing/                  ← rule-based Smart Routing v2 (no LLM)
│   ├── scoring/                  ← R_eff + F-G-R quality scoring
│   ├── search/                   ← keyword + semantic search
│   ├── stale/                    ← expired valid_until detection
│   ├── template/                 ← tera template engine
│   ├── validation/               ← depth-aware rules (30+ per kind)
│   └── workspace/                ← .forgeplan/ directory management
├── forgeplan-cli/                ← CLI binary (33 commands, clap derive)
└── forgeplan-mcp/                ← MCP server (26 tools, rmcp, stdio transport)
```

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
