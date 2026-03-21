# Forgeplan — Full Context for New Chat

> Этот файл содержит ВСЁ что нужно знать чтобы продолжить работу над Forgeplan в новом чате.
> Прочитай его целиком перед началом работы.

---

## 1. Что такое Forgeplan

**Forgeplan** — универсальная Rust-платформа (CLI + Desktop App) для ведения любого проекта от идеи до реализации. Помогает создавать, связывать и валидировать документы (PRD, Epic, Spec, RFC, ADR) с quality scoring и semantic search.

```
forgeplan init                          # создать .forgeplan/ в проекте
forgeplan new prd "Social Login"        # PRD из шаблона
forgeplan new rfc "OAuth2 Architecture" # RFC, linked to PRD
forgeplan status                        # dashboard + progress bars
forgeplan validate                      # проверить полноту
forgeplan search "authentication"       # semantic search
forgeplan graph                         # dependency graph (mermaid)
forgeplan score PRD-001                 # R_eff quality scoring
```

**Alias**: `fpl` (опциональный)

---

## 2. Формула

```
Forgeplan = Quint-code (decision engine, R_eff scoring, evidence decay)
          + BMAD (PRD workflow, 13-step validation, adversarial review)
          + OpenSpec (artifact DAG, delta-specs, custom schemas)
          + FPF (reasoning framework, ADI cycle, trust calculus)
          + git-adr (Rust CLI patterns, clap, templates)
          + LanceDB (embedded DB: tables + vectors в одном)
          + Tauri (desktop app: React UI + shared Rust core)
```

---

## 3. Ключевые решения (уже приняты)

| Решение | Выбор | Почему |
|---------|-------|--------|
| Язык | **Rust** | Type safety, single binary, git-adr reference |
| CLI framework | **clap** (derive) | Auto-completions, typed args |
| Database | **LanceDB** | Tables + vectors в одной embedded DB |
| Full-text search | **Tantivy** | Embedded Elasticsearch, pure Rust |
| Local embeddings | **ort** (ONNX Runtime) | BGE-M3 / all-MiniLM local |
| Desktop | **Tauri 2.0 + React** | ~10MB, shares Rust core |
| Templates | **tera** | Jinja2-compatible |
| Markdown | **pulldown-cmark** | Fast, CommonMark |
| MCP | **rmcp** | Official Rust MCP SDK |
| Название | **forgeplan** | Свободно на GitHub, crates.io, npm |

---

## 4. Артефакты (11 типов)

### Из Quint-code (6):
| Kind | Prefix | Описание |
|------|--------|----------|
| Note | `note-` | Микро-решение |
| ProblemCard | `prob-` | Проблема с контекстом |
| SolutionPortfolio | `sol-` | 2-3+ варианта (weakest link) |
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

---

## 5. Архитектура (Rust workspace)

```
forgeplan/
├── crates/
│   ├── forgeplan-core/       ← SHARED LIBRARY (вся логика)
│   │   ├── artifact/         ← types, parser, writer, store
│   │   ├── scoring/          ← R_eff = min(evidence_scores)
│   │   ├── validation/       ← BMAD 13-step rules
│   │   ├── search/           ← LanceDB vectors + Tantivy text
│   │   ├── progress/         ← checkbox parser + progress bars
│   │   ├── graph/            ← mermaid dependency graph
│   │   ├── embed/            ← ONNX local embeddings
│   │   ├── db/               ← LanceDB operations
│   │   ├── template/         ← tera engine
│   │   └── config/           ← .forgeplan/config.yaml
│   │
│   ├── forgeplan-cli/        ← CLI binary (clap + core)
│   ├── forgeplan-tauri/      ← Desktop app backend (Tauri + core)
│   └── forgeplan-mcp/        ← MCP server (Phase 5)
│
├── apps/
│   └── desktop/              ← React frontend (Tauri UI)
│       ├── Dashboard, Artifacts, Editor, Graph, Quality, Timeline
│
├── templates/                ← .md.tera шаблоны (embedded in binary)
├── docs/                     ← schemas, guides, references
└── sources/                  ← reference repos (read-only)
```

---

## 6. Data Model (ключевые структуры)

```rust
enum ArtifactKind {
    Note, ProblemCard, SolutionPortfolio,
    DecisionRecord, EvidencePack, RefreshReport,
    PRD, Epic, Spec, RFC, ADR,
}

struct Meta {
    id: String,               // "prd-20260321-001"
    kind: ArtifactKind,
    version: u32,
    status: Status,           // Draft, Active, Superseded, Deprecated, RefreshDue
    title: String,
    mode: Option<Mode>,       // note, tactical, standard, deep
    valid_until: Option<DateTime>,
    links: Vec<Link>,         // {target, relation: informs|based_on|supersedes|...}
    parent_epic: Option<String>,
}

struct Artifact {
    meta: Meta,
    body: String,             // markdown
    embedding: Option<Vec<f32>>,
}

// R_eff = min(evidence_scores) — weakest link, NEVER average
fn r_eff(evidence: &[EvidenceItem]) -> f64 { ... }
```

---

## 7. Storage: LanceDB + Markdown

- **LanceDB** = source of truth (structured tables + vector embeddings)
- **Markdown** = human-readable projections (git-tracked)
- Sync on every write: write to LanceDB → render markdown → save to `.forgeplan/`

```
.forgeplan/
├── config.yaml       ← project config
├── lance/            ← LanceDB (gitignore)
├── prds/             ← markdown (git-tracked)
├── epics/
├── specs/
├── rfcs/
├── adrs/
├── problems/
├── solutions/
├── decisions/
├── evidence/
├── notes/
└── refresh/
```

---

## 8. Phases (план реализации)

```
Phase 0  ████████████████████████  10/10  (100%)  Foundation & Research ✅
Phase 1  ██████████░░░░░░░░░░░░░░   5/12  ( 42%)  Schemas & Templates (IN PROGRESS)
Phase 2  ░░░░░░░░░░░░░░░░░░░░░░░░   0/8   (  0%)  Workflow & Integration
Phase 3  ░░░░░░░░░░░░░░░░░░░░░░░░   0/12  (  0%)  Rust CLI + LanceDB
Phase 4  ░░░░░░░░░░░░░░░░░░░░░░░░   0/7   (  0%)  Desktop App + AI
Phase 5  ░░░░░░░░░░░░░░░░░░░░░░░░   0/3   (  0%)  MCP Server
```

### Phase 1 — что осталось:
- [ ] Обогатить PRD шаблон из BMAD `create-prd/` (13 validation steps)
- [ ] Product Brief шаблон (lightweight PRD)
- [ ] Problem Card, Solution Portfolio, DDR шаблоны (из quint-code)
- [ ] DEPTH-CALIBRATION.md
- [ ] QUALITY-GATES.md
- [ ] GLOSSARY.md

### Phase 3 — Rust CLI (главное):
- [ ] `cargo init` workspace
- [ ] Artifact types (port quint-code `types.go` → Rust)
- [ ] R_eff scoring (port `reff.go` — 52 строки)
- [ ] LanceDB integration
- [ ] Template engine (tera + embedded)
- [ ] CLI commands (clap derive)
- [ ] Validator (BMAD rules)
- [ ] Progress tracker + graph builder

---

## 9. Key Patterns (20+ паттернов из исследования)

### Scoring (Quint-code):
- **R_eff = min(evidence_scores)** — trust = weakest link
- **Evidence Decay** — valid_until TTL, expired = 0.1 (weak, not absent)
- **CL penalty** — Congruence Level: CL3=0.0, CL2=0.1, CL1=0.4, CL0=0.9
- **DerivedStatus** — UNDERFRAMED → FRAMED → EXPLORING → COMPARED → DECIDED → APPLIED

### Workflow (BMAD):
- **Contextual chain** — each phase output = next phase input
- **Adversarial Review** — reviewer MUST find problems; 0 = re-review
- **Quick Flow vs Full Path** — adaptive depth by complexity
- **13-step PRD validation** — discovery → density → measurability → traceability

### Pipeline (OpenSpec):
- **Delta-specs** — describe ONLY changes (ADDED/MODIFIED/REMOVED)
- **Artifact DAG** — proposal → specs → design → tasks
- **Verify/Archive** — formal verification before closing

### Reasoning (FPF):
- **ADI cycle** — Abduction (3+ hypotheses) → Deduction → Induction
- **F-G-R Trust Calculus** — Formality, Granularity, Reliability
- **Transformer Mandate** — agent generates options, human decides

---

## 10. Reference Repos (в `sources/`)

| Repo | Язык | Что смотреть | Когда |
|------|------|-------------|-------|
| **quint-code** | Go | `src/mcp/internal/artifact/types.go` — data model | Проектирование artifact types |
| | | `src/mcp/internal/reff/reff.go` — R_eff scoring (52 LOC) | Scoring module |
| | | `src/mcp/schema.sql` — SQLite schema (9 tables) | DB design |
| | | `src/mcp/cmd/commands/*.md` — slash commands | CLI UX |
| | | `CLAUDE.md` — thinking principles | Design philosophy |
| **git-adr** | Rust | `src/cli/` — clap CLI commands | Rust CLI patterns |
| | | `src/core/templates.rs` — template engine | Template rendering |
| | | `src/core/adr.rs` — ADR model | Artifact CRUD |
| **OpenSpec** | TS | `src/core/archive.ts` — archive/merge | Delta-spec logic |
| | | `schemas/` — artifact type definitions | Custom schemas |
| | | `src/core/parsers/` — markdown parsing | Parser patterns |
| **BMAD** | MD | `src/bmm-skills/2-plan-workflows/create-prd/` | **PRD creation workflow!** |
| | | `steps-v/step-v-01..13` — 13 validation steps | Validation rules |
| | | `bmad-create-prd/steps-c/` — creation steps | PRD generation |
| **ccpm** | MD | `skill/` — Claude Code project management | MCP integration |

---

## 11. Reference Docs (в `docs/ref/`)

| Документ | Приоритет | Что взять |
|----------|-----------|-----------|
| OpenSpec Методичка | HIGH | Delta-specs, artifact DAG, verify/archive |
| BMAD Гайд | HIGH | 4-phase chain, 9 agents, adversarial review |
| Методология FPF | HIGH | ADI cycle, DDR template, evidence decay |
| Методика Quint-code | HIGH | R_eff, verification gates, depth calibration |
| OpenSpec ExtraBoost | MEDIUM | Review checklists, 4-agent pipeline |
| Контекстная инженерия | MEDIUM | Context budget, U-shaped attention |
| Дизайн Фаза Модуль | MEDIUM | Design Phase 5 sub-phases |
| Анализ видео | MEDIUM | "NOT a state machine" insight |

---

## 12. Файлы проекта — что где

| Файл | Что содержит | Когда смотреть |
|------|-------------|----------------|
| `VISION.md` | **ВСЯ архитектура**: 17 секций, data model, tech stack, screens, phases | Проектирование, архитектурные решения |
| `PLAN.md` | **49 задач** в 5 фазах с progress bars | Планирование, tracking |
| `TODO.md` | **Текущие приоритеты** P0/P1/P2 | Что делать сейчас |
| `COMPLETENESS-CHECK.md` | **52 компонента** по 10 слоям, gap analysis | Проверка что ничего не забыли |
| `SOURCES.md` | Карта ВСЕХ источников (skills, commands, agents, repos) | Поиск нужного инструмента |
| `README.md` | Quick start | Onboarding |
| `docs/schemas/PRD-SCHEMA.md` | Правила PRD: обязательные секции, validation | Создание PRD |
| `docs/schemas/EPIC-SCHEMA.md` | Правила Epic: aggregated progress, children | Создание Epic |
| `docs/schemas/SPEC-SCHEMA.md` | Правила Spec: API contracts, data models | Создание Spec |
| `docs/guides/ARTIFACT-MODEL.md` | Иерархия: Epic→PRD→Spec→RFC→ADR + lifecycle | Понимание артефактов |
| `docs/guides/PRD-RFC-ADR-FLOW.md` | Decision tree: какой документ создать | Workflow decisions |
| `docs/references/REF-DOCS-ANALYSIS.md` | Анализ 10 документов + 10 паттернов | Methodology reference |
| `docs/references/SKILLS-AUDIT.md` | 52 skills по 10 слоям + gaps | Tool selection |
| `templates/*/` | 5 готовых шаблонов | Создание документов |

---

## 13. Desktop App — экраны

| Screen | Что показывает |
|--------|---------------|
| Dashboard | Progress bars, R_eff scores, stale alerts, recent |
| Artifacts | Table + cards, filter, semantic search |
| Editor | Split-pane markdown + live preview |
| Graph | Interactive dependency graph (react-flow) |
| Timeline | Gantt-like Epic → PRD → RFC phases |
| Quality | R_eff dashboard, evidence decay, verification gates |

---

## 14. Canonical Pipeline (подтверждён 4 источниками)

```
Design Phase (FPF/ADI)
    ↓
Spec Phase (OpenSpec/BMAD)
    ↓
Code Phase (Claude Code / IDE)
    ↓
Verify Phase (CI/CD)
```

Pipeline = **guideline, NOT rigid sequence** (FPF автор: "произвольные траектории").

---

## 15. Depth Calibration (из Quint-code)

| Complexity | Depth | Что создаём |
|-----------|-------|-------------|
| Quick fix, 1 файл | **Tactical** | Note или ничего |
| Фича 1-3 дня | **Standard** | PRD (tactical) → RFC |
| Новый модуль, 1-2 нед | **Deep** | PRD → SPEC → RFC → ADR |
| Подсистема, кросс-команда | **Critical** | Epic → PRD[] → SPEC[] → RFC[] → ADR[] |

---

## 16. Что делать дальше (приоритеты)

### Немедленно (Phase 1 завершение):
1. Изучить BMAD `sources/BMAD-METHOD/src/bmm-skills/2-plan-workflows/create-prd/` — взять 13 validation steps
2. Дописать шаблоны: Problem Card, Solution Portfolio, DDR
3. Написать GLOSSARY.md

### После Phase 1 → Phase 3 (Rust CLI):
1. `cargo init` workspace (crates/forgeplan-core + crates/forgeplan-cli)
2. Port `types.go` → `artifact/types.rs`
3. Port `reff.go` → `scoring/reff.rs` (52 строки)
4. LanceDB integration
5. CLI scaffold (clap)
6. Template engine (tera)

### Параллельно Phase 2 (Claude Code integration):
- Расширить `/write-doc` (prd, epic, spec)
- Создать `/prd` slash command

---

## 17. Советы для нового чата

1. **Начни с**: `Прочитай frameworks/CONTEXT.md и frameworks/VISION.md — это проект Forgeplan`
2. **Для архитектуры**: смотри `VISION.md` секции 5-9
3. **Для планирования**: смотри `PLAN.md`
4. **Для текущих задач**: смотри `TODO.md`
5. **Для reference code**: смотри `sources/quint-code/src/mcp/internal/` (data model, R_eff)
6. **Для PRD workflow**: смотри `sources/BMAD-METHOD/src/bmm-skills/2-plan-workflows/create-prd/`
7. **Для Rust CLI patterns**: смотри `sources/git-adr/src/`
8. **FPF skill**: уже установлен как `.claude/skills/fpf-simple/` — используй `/fpf-simple` для reasoning
9. **Язык**: Rust (preferred). Desktop: Tauri + React. DB: LanceDB.
10. **Название**: `forgeplan` (свободно на GitHub, crates.io, npm). Alias: `fpl`.

---

## 18. Non-Goals

- НЕ project management (не Jira/Linear)
- НЕ CI/CD
- НЕ SaaS (local-first, single binary)
- НЕ code generator
- НЕ real-time collaboration (git для sync)
