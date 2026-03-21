# VISION — Forgeplan

> Финальная идея, все слои, все модули, все решения.
> Этот документ — source of truth перед началом проектирования.

---

## 1. Что это

**Forgeplan** — Rust platform (CLI + Desktop App) для ведения любого проекта
от идеи до реализации через структурированные артефакты с quality scoring,
semantic search и evidence tracking.

**CLI**: `forgeplan` (alias: `fpl`)
**Desktop**: Tauri + React (shared Rust core)

**Формула**: Quint-code (decision engine) + BMAD (PRD workflow) + OpenSpec (artifact pipeline) + FPF (reasoning) + git-adr (Rust CLI) + LanceDB (vectors) + Tauri (desktop).

---

## 2. Проблема

**Без Forgeplan**:
- Документы (PRD, RFC, ADR) создаются ad-hoc, без стандартов
- Нет связей между артефактами (PRD-001 → RFC-042 → ADR-007)
- Нет tracking прогресса по артефактам
- Решения не записываются с rationale, устаревают молча
- Нет quality gates — документы не валидируются на полноту
- Нет semantic search — "найди всё про authentication" невозможно
- Каждый проект изобретает свой процесс заново

**С Forgeplan**:
- `forgeplan init` — один раз, процесс готов
- `forgeplan new prd "Social Login"` — шаблон + автонумерация + линковка
- `forgeplan validate` — проверка полноты, ссылок, TTL
- `forgeplan status` — dashboard всех артефактов с progress bars
- `forgeplan score` — R_eff quality scoring по evidence
- `forgeplan search "auth"` — semantic search по всем артефактам
- `forgeplan graph` — dependency graph в mermaid
- Desktop App — visual dashboard, editor, interactive graph

---

## 3. Product: CLI + Desktop + MCP

```
                     Forgeplan
                         │
           ┌─────────────┼─────────────┐
           │             │             │
      CLI (Rust)    Desktop App    MCP Server
      `forgeplan`   (Tauri+React)  (Phase 5)
           │             │
           └──────┬──────┘
                  │
           ┌──────┴──────┐
           │  Core Lib   │  ← forgeplan-core (shared)
           │  (Rust)     │
           ├─────────────┤
           │ LanceDB     │  ← tables + vectors + search
           │ Tantivy     │  ← full-text search
           │ ort (ONNX)  │  ← local embeddings (BGE-M3)
           │ tera        │  ← templates
           │ clap        │  ← CLI only
           │ pulldown-cmark │ ← markdown parser
           └─────────────┘
```

---

## 4. Use Cases

| # | Сценарий | Flow |
|---|----------|------|
| 1 | **Новая фича** | `forgeplan new prd` → `forgeplan new spec` → `forgeplan new rfc` → sprint |
| 2 | **Рефакторинг** | `forgeplan new adr` → `forgeplan new rfc` → sprint |
| 3 | **Decompose monolith** | `forgeplan new epic` → N × `forgeplan new prd` → N × RFC |
| 4 | **Миграция** (framework, DB) | `forgeplan new epic` → `forgeplan new adr` → RFC per phase |
| 5 | **Новый проект с нуля** | `forgeplan init` → epic → PRD → spec → RFC → sprint |
| 6 | **Технический долг** | `forgeplan new problem` → `forgeplan new adr` → RFC |
| 7 | **API Design** | `forgeplan new spec` → `forgeplan new rfc` |
| 8 | **Security Audit** | analysis → `forgeplan new adr` → RFC remediation |
| 9 | **Incident Response** | `forgeplan new problem` → `forgeplan new adr` (root cause) → RFC (fix) |
| 10 | **Оценка + Roadmap** | `forgeplan new epic` per quarter → PRDs → track progress |

---

## 5. Artifact Types (11 kinds)

### From Quint-code (6):
| Kind | ID Prefix | Описание |
|------|-----------|----------|
| Note | `note-` | Микро-решение, заметка |
| ProblemCard | `prob-` | Проблема с контекстом |
| SolutionPortfolio | `sol-` | 2-3+ варианта решения |
| DecisionRecord | `dec-` | DDR: решение + invariants + rollback |
| EvidencePack | `evid-` | Доказательства (тесты, benchmarks) |
| RefreshReport | `ref-` | Переоценка устаревших решений |

### New for Forgeplan (5):
| Kind | ID Prefix | Описание |
|------|-----------|----------|
| **PRD** | `prd-` | Product Requirements Document |
| **Epic** | `epic-` | Стратегическая инициатива (группирует PRD[]) |
| **Spec** | `spec-` | Формальная спецификация (API, data model) |
| **RFC** | `rfc-` | Архитектурное предложение с фазами |
| **ADR** | `adr-` | Architecture Decision Record |

---

## 6. Storage: LanceDB + Markdown

### LanceDB — единое хранилище (tables + vectors)

```rust
// LanceDB = structured data + vector search в одной embedded DB
// Заменяет SQLite + отдельный vector DB

let artifacts = db.create_table("artifacts", schema![
    id: String,
    kind: String,              // PRD, RFC, ADR, Epic...
    status: String,
    title: String,
    body: String,              // markdown content
    parent_epic: Option<String>,
    valid_until: Option<DateTime>,
    r_eff_score: f64,
    created_at: DateTime,
    updated_at: DateTime,
    embedding: Vector(384),    // ← semantic search built-in!
]).await?;

// Hybrid search: structured + semantic в одном запросе
let results = artifacts
    .search("OAuth authentication patterns")   // vector search
    .filter("kind = 'PRD' AND status = 'active'")  // SQL-like filter
    .limit(10)
    .execute().await?;
```

### Directory Structure
```
.forgeplan/
├── config.yaml          ← project config
├── lance/               ← LanceDB storage (tables + vectors)
│   ├── artifacts.lance
│   ├── evidence.lance
│   └── relations.lance
├── prds/                ← markdown projections (git-tracked)
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

**Dual storage**: LanceDB = source of truth (structured + vectors). Markdown = human-readable projections (git-tracked). Sync on every write.

---

## 7. Data Model (Rust)

### Core: Artifact
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
enum ArtifactKind {
    // From Quint-code
    Note, ProblemCard, SolutionPortfolio,
    DecisionRecord, EvidencePack, RefreshReport,
    // New for Forgeplan
    PRD, Epic, Spec, RFC, ADR,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Status {
    Draft, Active, Superseded, Deprecated, RefreshDue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Link {
    target: String,          // artifact ID
    relation: LinkType,      // informs, based_on, supersedes, contradicts, refines
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Meta {
    id: String,
    kind: ArtifactKind,
    version: u32,
    status: Status,
    title: String,
    context: Option<String>,
    mode: Option<Mode>,              // note, tactical, standard, deep
    valid_until: Option<NaiveDateTime>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    links: Vec<Link>,
    parent_epic: Option<String>,
}

#[derive(Debug, Clone)]
struct Artifact {
    meta: Meta,
    body: String,            // markdown content
    embedding: Option<Vec<f32>>,  // computed on save
}
```

### Evidence & Scoring (from Quint-code reff.go)
```rust
struct EvidenceItem {
    id: String,
    evidence_type: EvidenceType,      // measurement, test, benchmark, audit
    verdict: Verdict,                 // supports(1.0), weakens(0.5), refutes(0.0)
    congruence_level: u8,             // 0-3 (CL penalty: 0.9/0.4/0.1/0.0)
    formality_level: u8,              // 0-9
    valid_until: Option<NaiveDateTime>,
}

/// R_eff = min(evidence_scores) — weakest link, never average
fn r_eff(evidence: &[EvidenceItem]) -> f64 {
    evidence.iter()
        .map(|e| score_evidence(e))
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0)
}

fn score_evidence(e: &EvidenceItem) -> f64 {
    if expired(e.valid_until) { return 0.1; }  // stale, not absent
    let base = e.verdict.to_score();
    let penalty = cl_penalty(e.congruence_level);
    (base - penalty).max(0.0)
}
```

---

## 8. CLI Commands

### Core (Phase 3)
| Command | Описание |
|---------|----------|
| `forgeplan init` | Создать `.forgeplan/` workspace + LanceDB |
| `forgeplan new <kind> <title>` | Создать артефакт из шаблона |
| `forgeplan list [kind]` | Список артефактов |
| `forgeplan status` | Dashboard с progress bars + R_eff scores |
| `forgeplan show <id>` | Показать артефакт |
| `forgeplan validate [id]` | Проверить полноту (BMAD rules) |
| `forgeplan link <from> <to> <type>` | Связать артефакты |
| `forgeplan graph` | Dependency graph (mermaid) |
| `forgeplan score [id]` | R_eff quality scoring |

### Search (Phase 4)
| Command | Описание |
|---------|----------|
| `forgeplan search <query>` | Hybrid search (text + semantic) |
| `forgeplan similar <id>` | Артефакты похожие на данный |
| `forgeplan suggest --for <id>` | AI suggestions based on similar decisions |

### Extended (Phase 4-5)
| Command | Описание |
|---------|----------|
| `forgeplan coverage` | Module coverage tracking |
| `forgeplan refresh` | Найти stale артефакты (evidence decay) |
| `forgeplan export <format>` | Export (markdown, json, html) |
| `forgeplan serve` | MCP server mode |
| `forgeplan app` | Запустить Desktop App |

---

## 9. Architecture: Rust Workspace

```
forgeplan/
├── Cargo.toml                    ← workspace root
│
├── crates/
│   ├── forgeplan-core/           ← SHARED LIBRARY (вся логика)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── artifact/         ← types, parser, writer, store
│   │   │   │   ├── mod.rs
│   │   │   │   ├── types.rs      ← ArtifactKind, Meta, Link, Status
│   │   │   │   ├── parser.rs     ← YAML frontmatter + markdown body
│   │   │   │   ├── writer.rs     ← write to .forgeplan/ + LanceDB
│   │   │   │   └── store.rs      ← CRUD operations
│   │   │   ├── scoring/          ← from quint-code reff.go
│   │   │   │   ├── mod.rs
│   │   │   │   ├── reff.rs       ← R_eff = min(evidence_scores)
│   │   │   │   └── decay.rs      ← TTL-based evidence decay
│   │   │   ├── validation/       ← from BMAD create-prd/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── completeness.rs
│   │   │   │   └── rules.rs      ← per-kind validation
│   │   │   ├── search/           ← LanceDB + Tantivy
│   │   │   │   ├── mod.rs
│   │   │   │   ├── semantic.rs   ← vector search via LanceDB
│   │   │   │   └── fulltext.rs   ← Tantivy full-text
│   │   │   ├── progress/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── parser.rs     ← checkbox parser
│   │   │   │   └── bars.rs       ← ASCII progress bars
│   │   │   ├── graph/
│   │   │   │   ├── mod.rs
│   │   │   │   └── mermaid.rs    ← dependency graph
│   │   │   ├── embed/            ← local embeddings
│   │   │   │   └── mod.rs        ← ONNX Runtime (ort) + BGE-M3
│   │   │   ├── db/               ← LanceDB operations
│   │   │   │   └── mod.rs
│   │   │   ├── template/
│   │   │   │   └── mod.rs        ← tera + embedded templates
│   │   │   └── config/
│   │   │       └── mod.rs        ← .forgeplan/config.yaml
│   │   ├── templates/            ← embedded .md.tera files
│   │   │   ├── prd.md.tera
│   │   │   ├── epic.md.tera
│   │   │   ├── spec.md.tera
│   │   │   ├── rfc.md.tera
│   │   │   ├── adr.md.tera
│   │   │   ├── problem.md.tera
│   │   │   ├── solution.md.tera
│   │   │   └── decision.md.tera
│   │   └── Cargo.toml
│   │
│   ├── forgeplan-cli/            ← CLI BINARY
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   └── commands/
│   │   │       ├── mod.rs
│   │   │       ├── init.rs
│   │   │       ├── new.rs
│   │   │       ├── list.rs
│   │   │       ├── status.rs
│   │   │       ├── validate.rs
│   │   │       ├── search.rs
│   │   │       ├── link.rs
│   │   │       ├── graph.rs
│   │   │       └── score.rs
│   │   └── Cargo.toml
│   │
│   ├── forgeplan-tauri/          ← DESKTOP APP (Tauri backend)
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   └── commands.rs       ← Tauri IPC commands → core
│   │   ├── tauri.conf.json
│   │   └── Cargo.toml
│   │
│   └── forgeplan-mcp/            ← MCP SERVER (Phase 5)
│       ├── src/
│       │   ├── main.rs
│       │   ├── server.rs
│       │   └── tools.rs
│       └── Cargo.toml
│
├── apps/
│   └── desktop/                  ← React frontend for Tauri
│       ├── src/
│       │   ├── App.tsx
│       │   ├── pages/
│       │   │   ├── Dashboard.tsx      ← status, progress bars, R_eff
│       │   │   ├── Artifacts.tsx      ← list, filter, semantic search
│       │   │   ├── ArtifactView.tsx   ← single artifact viewer
│       │   │   ├── Editor.tsx         ← markdown editor + preview
│       │   │   ├── Graph.tsx          ← interactive dependency graph
│       │   │   ├── Timeline.tsx       ← Epic → PRD → RFC phases
│       │   │   ├── Quality.tsx        ← R_eff scores, decay alerts
│       │   │   └── Settings.tsx       ← config
│       │   └── components/
│       │       ├── ProgressBar.tsx
│       │       ├── ArtifactCard.tsx
│       │       ├── SearchBar.tsx
│       │       └── MermaidRenderer.tsx
│       ├── package.json
│       └── vite.config.ts
│
├── templates/                    ← source templates (also embedded in binary)
│   ├── prd/_TEMPLATE.md
│   ├── epic/_TEMPLATE.md
│   ├── spec/_TEMPLATE.md
│   ├── rfc/_TEMPLATE.md
│   └── adr/_TEMPLATE.md
│
├── docs/                         ← project documentation
│   ├── schemas/
│   ├── guides/
│   └── references/
│
└── sources/                      ← reference implementations
    ├── quint-code/
    ├── OpenSpec/
    ├── BMAD-METHOD/
    ├── git-adr/
    ├── adr-tools/
    └── ccpm/
```

---

## 10. Technology Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| **Language** | Rust | Type safety, single binary, shared core for CLI + Desktop |
| **CLI** | clap (derive) | Auto-completions, man pages, typed args |
| **Desktop** | Tauri 2.0 + React | ~10MB binary, native APIs, shares Rust core |
| **Frontend** | React + TypeScript + Tailwind | Fast iteration, component ecosystem |
| **Database** | LanceDB | Embedded, tables + vectors, zero-config, Apache Arrow |
| **Full-text** | Tantivy | Embedded Elasticsearch alternative, pure Rust |
| **Embeddings** | ort (ONNX Runtime) | Local BGE-M3 / all-MiniLM-L6-v2, no API calls |
| **Templates** | tera | Jinja2-compatible |
| **Markdown** | pulldown-cmark | Fast, CommonMark compliant |
| **YAML** | serde_yaml | serde ecosystem |
| **Diagrams** | Mermaid | GitHub/GitLab renderable, react-mermaid for desktop |
| **Graph viz** | react-flow | Interactive dependency graph in desktop app |
| **MCP** | rmcp | Official Rust MCP SDK |

---

## 11. Desktop App Screens

| Screen | Purpose | Key Features |
|--------|---------|--------------|
| **Dashboard** | Overview | Progress bars, R_eff scores, stale alerts, recent artifacts |
| **Artifacts** | Browse & Search | Table + cards, filter by kind/status, semantic search bar |
| **Artifact View** | Read | Rendered markdown, linked artifacts, evidence, scores |
| **Editor** | Create/Edit | Split-pane markdown editor + live preview, template picker |
| **Graph** | Dependencies | Interactive react-flow graph, click to navigate, mermaid export |
| **Timeline** | Roadmap | Gantt-like view for Epic → PRD → RFC phases |
| **Quality** | Scoring | R_eff dashboard, evidence decay timeline, verification gates |
| **Settings** | Config | Project config, embedding model selection, themes |

---

## 12. Key Patterns (из исследования)

### From Quint-code (decision engine):
1. R_eff = min(evidence_scores) — trust = weakest link
2. Evidence Decay — valid_until TTL, stale = 0.1
3. CL penalty — Congruence Level: 0.0/0.1/0.4/0.9
4. Artifact lifecycle — active → refresh_due → superseded/deprecated
5. DerivedStatus — UNDERFRAMED → FRAMED → EXPLORING → COMPARED → DECIDED → APPLIED
6. Module coverage — tracks which code modules have decisions vs "blind"
7. Drift detection — file hash baseline, detect code changes under decisions
8. Transformer Mandate — agent generates options, human decides

### From BMAD (PRD workflow):
9. 4-phase contextual chain — Analysis → Planning → Solutioning → Implementation
10. Adversarial Review — reviewer MUST find problems; 0 = re-review
11. Quick Flow vs Full Path — adaptive depth based on complexity
12. 13-step PRD validation — discovery → density → measurability → traceability → completeness

### From OpenSpec (artifact pipeline):
13. Delta-specifications — describe ONLY changes (ADDED/MODIFIED/REMOVED)
14. Artifact DAG — proposal → specs → design → tasks
15. Custom schemas — extensible artifact types
16. Verify/Archive cycle — formal verification before closing

### From FPF (reasoning foundation):
17. ADI cycle — Abduction (3+ hypotheses) → Deduction → Induction
18. Bounded Contexts — explicit semantic boundaries
19. F-G-R Trust Calculus — Formality, Granularity, Reliability
20. Pareto Front — trade-off visualization for decisions

### From git-adr (Rust CLI patterns):
21. clap derive — CLI argument parsing
22. Template rendering — Rust embedded templates
23. Export formats — markdown, JSON, wiki

---

## 13. Implementation Phases

```
Phase 3A: Rust Core + CLI
  ├── forgeplan-core: artifact types, parser, writer, store
  ├── forgeplan-cli: init, new, list, status, validate, link, graph, score
  ├── LanceDB: tables for artifacts, relations, evidence
  └── Tantivy: full-text search

Phase 3B: Local AI
  ├── ort (ONNX): local embeddings (all-MiniLM-L6-v2 → BGE-M3)
  ├── LanceDB vectors: semantic search on artifacts
  └── forgeplan search: hybrid (text + semantic)

Phase 4A: Desktop App
  ├── Tauri 2.0 shell: wraps forgeplan-core
  ├── React UI: Dashboard, Artifacts, Editor, Graph, Quality
  └── react-flow: interactive dependency visualization

Phase 4B: Advanced AI
  ├── Local reranker (BGE-reranker-v2-m3 via ONNX)
  ├── LLM integration: generate PRD drafts, suggest links
  └── Auto-capture: extract decisions from conversation

Phase 5: MCP Server
  ├── forgeplan-mcp: MCP protocol handler
  ├── Tools: create/search/validate artifacts from Claude Code
  └── Claude Code skill/commands integration
```

---

## 14. Reference Implementations

| Repo | Язык | Что берём |
|------|------|-----------|
| **quint-code** | Go | Data model, R_eff, SQLite schema, MCP, evidence decay |
| **git-adr** | Rust | CLI structure, clap patterns, templates, git integration |
| **OpenSpec** | TypeScript | Artifact DAG, delta-specs, custom schemas |
| **BMAD** | MD | PRD workflow (13 steps), validation, agent specialization |
| **adr-tools** | Bash | Original ADR CLI (reference) |
| **ccpm** | MD | Claude Code project management patterns |

---

## 15. Open Questions

1. **Где хранить `.forgeplan/`?** → Корень проекта (like `.git/`).
   - `lance/` (DB) → `.gitignore`
   - Markdown артефакты → git-tracked

2. **Embedding model default?** → `all-MiniLM-L6-v2` (22MB, fast, good enough).
   Upgrade path: `BGE-M3` (1.2GB, multilingual, better quality).

3. **Tauri 2.0 when?** → Phase 4A, после CLI стабилен.

4. **LanceDB vs SQLite?** → LanceDB primary (tables + vectors).
   SQLite fallback если vectors не нужны (`forgeplan init --no-vectors`).

---

## 16. Non-Goals

- НЕ project management tool (не заменяем Jira/Linear)
- НЕ CI/CD (не запускаем тесты/деплой)
- НЕ real-time collaboration (git для sync)
- НЕ code generator (документы и шаблоны)
- НЕ SaaS (local-first, single binary)

---

## 17. Success Criteria

| Metric | Target |
|--------|--------|
| `forgeplan init` → first artifact | < 30 секунд |
| `forgeplan validate` coverage | 100% обязательных секций |
| `forgeplan status` render | < 100ms |
| `forgeplan search` (semantic) | < 500ms on 1000 artifacts |
| Artifact types | 11 kinds |
| Template quality | BMAD 13-step validation pass |
| CLI binary size | < 15MB (with ONNX model < 40MB) |
| Desktop app size | < 50MB |
| Test coverage | > 80% |
