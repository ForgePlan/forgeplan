---
id: RFC-003
title: "LanceDB Integration — async storage, schema, migration"
status: Draft
author: explosovebit
created: 2026-03-21
updated: 2026-03-21
prd: PRD-001
depth: deep
---

# RFC-003: LanceDB Integration

## Progress

```
Phase D  ████████████████░░░░░░░░  3/5   ( 60%)  LanceDB Integration
─────────────────────────────────────────────────
TOTAL                               3/5   ( 60%)
```

---

## Summary

Интеграция LanceDB как primary storage для Forgeplan CLI. Все CRUD операции через LanceDB, markdown файлы = git-tracked projections. Async runtime (tokio) для всего core. Адаптация quint-code schema.sql (9 таблиц → 3 LanceDB таблицы).

## Motivation

ADR-002 определяет LanceDB = source of truth. Текущий file-based store работает, но не поддерживает:
- Structured queries (filter by kind + status + date range)
- Vector search (semantic similarity между артефактами)
- Atomic operations (concurrent read/write safety)
- Evidence → Artifact relationships через foreign keys

Без LanceDB: search = regex grep (O(n) scan), нет semantic search, нет structured queries, нет Phase 4 (Desktop App с rich queries).

## Goals

- Определить LanceDB schema (3 таблицы: artifacts, evidence, relations)
- Описать async migration path (sync → tokio)
- Определить ArtifactStore trait (абстракция над storage backend)
- Описать markdown projection pipeline (LanceDB write → markdown render)
- Обратная совместимость: `forgeplan init` мигрирует существующие markdown файлы

## Non-Goals

- Vector embeddings / semantic search (Phase 4, requires ONNX)
- Desktop App integration (Phase 4, Tauri)
- MCP server (Phase 5)
- Migration tool для v0.1.0 → v0.2.0 workspaces (отдельная задача)

---

## Options Considered

### Option A: Full async migration (выбран)

**Description**: Весь core становится async. tokio runtime. LanceDB = primary store. Markdown = projection.

**Pros**: Чистая архитектура. Готовность к Phase 4 (Tauri async). Нет sync/async boundary проблем.

**Cons**: Ломает все 97 тестов (нужен `#[tokio::test]`). Увеличивает compile time.

### Option B: Sync wrapper вокруг async

**Description**: LanceDB вызывается через `tokio::runtime::Runtime::block_on()` из sync кода.

**Pros**: Минимальные изменения в существующем коде.

**Cons**: Nested runtime panic. Плохой DX. Не работает с Tauri (который тоже async).

### Option C: LanceDB только для search, file store для CRUD

**Description**: File store остаётся primary. LanceDB = read-only index для search.

**Pros**: Минимальные изменения. Инкрементально.

**Cons**: Два source of truth. Sync проблемы. Не соответствует ADR-002.

## Trade-off Analysis

| Критерий | Full async (A) | Sync wrapper (B) | Search-only (C) |
|----------|---------------|-------------------|-----------------|
| ADR-002 compliance | Full | Partial | No |
| Code changes | High | Low | Low |
| Tauri readiness | Yes | No (nested runtime) | No |
| Correctness | Best | Runtime panics | Dual source |
| Future maintenance | Simple | Complex | Complex |

---

## Proposed Direction

**Option A: Full async migration**. Соответствует ADR-002, готовит к Phase 4 (Tauri), чистая архитектура.

---

## Architecture

### LanceDB Schema (3 таблицы)

Адаптация quint-code schema.sql (9 таблиц) → 3 LanceDB таблицы. Quint-code таблицы `work_records`, `audit_log`, `waivers`, `predictions`, `fpf_state`, `characteristics` не нужны для MVP CLI.

#### Table: `artifacts`

```
┌─────────────┬───────────────────┬───────────────────────────────────┐
│ Column       │ Type              │ Description                       │
├─────────────┼───────────────────┼───────────────────────────────────┤
│ id          │ Utf8              │ PK: "PRD-001", "RFC-002"          │
│ kind        │ Utf8              │ "prd", "rfc", "adr", "epic", etc. │
│ status      │ Utf8              │ "draft", "active", "superseded"   │
│ title       │ Utf8              │ Human-readable title              │
│ body        │ Utf8 (Large)      │ Markdown content (after ---)      │
│ depth       │ Utf8              │ "tactical", "standard", "deep"    │
│ author      │ Utf8 (nullable)   │ Author name                       │
│ parent_epic │ Utf8 (nullable)   │ FK: parent epic ID                │
│ r_eff_score │ Float64           │ Cached R_eff score                │
│ valid_until │ Utf8 (nullable)   │ ISO date for evidence decay       │
│ created_at  │ Utf8              │ ISO datetime                      │
│ updated_at  │ Utf8              │ ISO datetime                      │
│ embedding   │ FixedSizeList(384)│ Vector for semantic search        │
└─────────────┴───────────────────┴───────────────────────────────────┘
```

#### Table: `evidence`

```
┌──────────────────┬───────────────┬──────────────────────────────────┐
│ Column            │ Type          │ Description                      │
├──────────────────┼───────────────┼──────────────────────────────────┤
│ id               │ Utf8          │ PK: "EVID-001"                   │
│ artifact_id      │ Utf8          │ FK: linked artifact              │
│ evidence_type    │ Utf8          │ "measurement", "test", etc.      │
│ verdict          │ Utf8          │ "supports", "weakens", "refutes" │
│ congruence_level │ Int32         │ 0-3                              │
│ valid_until      │ Utf8 (null)   │ ISO date                         │
│ content          │ Utf8          │ Evidence description             │
│ created_at       │ Utf8          │ ISO datetime                     │
└──────────────────┴───────────────┴──────────────────────────────────┘
```

#### Table: `relations`

```
┌────────────────┬──────────┬───────────────────────────────────────┐
│ Column          │ Type     │ Description                           │
├────────────────┼──────────┼───────────────────────────────────────┤
│ source_id      │ Utf8     │ FK: source artifact                   │
│ target_id      │ Utf8     │ FK: target artifact                   │
│ relation_type  │ Utf8     │ "informs", "based_on", "supersedes"   │
│ created_at     │ Utf8     │ ISO datetime                          │
└────────────────┴──────────┴───────────────────────────────────────┘
```

### Async Migration

```rust
// Before (sync):
pub fn list_artifacts(workspace: &Path) -> anyhow::Result<Vec<ArtifactSummary>>

// After (async):
pub async fn list_artifacts(db: &Database) -> anyhow::Result<Vec<ArtifactSummary>>

// CLI main:
#[tokio::main]
async fn main() -> anyhow::Result<()> { ... }

// Tests:
#[tokio::test]
async fn test_list_artifacts() { ... }
```

### ArtifactStore Trait (abstraction)

```rust
#[async_trait]
pub trait ArtifactStore {
    async fn create(&self, artifact: &NewArtifact) -> Result<ArtifactSummary>;
    async fn get(&self, id: &str) -> Result<Option<Artifact>>;
    async fn list(&self, filter: &ArtifactFilter) -> Result<Vec<ArtifactSummary>>;
    async fn update(&self, id: &str, update: &ArtifactUpdate) -> Result<()>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn add_link(&self, source: &str, target: &str, relation: &str) -> Result<()>;
    async fn get_links(&self, id: &str) -> Result<Vec<Link>>;
    async fn search(&self, query: &str, kind: Option<&str>) -> Result<Vec<SearchHit>>;
}
```

### LanceDB Store Implementation

```rust
pub struct LanceStore {
    db: Database,
    artifacts_table: Table,
    evidence_table: Table,
    relations_table: Table,
    workspace_path: PathBuf,  // for markdown projections
}

impl LanceStore {
    pub async fn open(workspace: &Path) -> Result<Self> {
        let lance_dir = workspace.join("lance");
        let db = lancedb::connect(lance_dir.to_str().unwrap()).execute().await?;
        // Open or create tables...
        Ok(Self { db, ... })
    }
}
```

### Markdown Projection Pipeline

```
User: forgeplan new prd "Feature X"
  → LanceStore::create(artifact)
    → Insert into LanceDB artifacts table
    → Render markdown: frontmatter (from record) + body (from template)
    → Write to .forgeplan/prds/PRD-001-feature-x.md
    → Return ArtifactSummary

User: forgeplan list
  → LanceStore::list(filter)
    → SELECT id, kind, status, title FROM artifacts WHERE ...
    → Return Vec<ArtifactSummary> (no file I/O!)

User: forgeplan validate PRD-001
  → LanceStore::get("PRD-001")
    → SELECT * FROM artifacts WHERE id = 'PRD-001'
    → Return Artifact with body
    → validate(body, frontmatter, kind, depth)
```

### Data Flow

```
                    ┌─────────────┐
                    │  CLI Layer   │
                    │  (clap)      │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │  Core Layer  │
                    │  (async)     │
                    └──────┬──────┘
                           │
                ┌──────────▼──────────┐
                │    ArtifactStore     │ ← trait
                │    (LanceStore)      │ ← impl
                └──────────┬──────────┘
                           │
              ┌────────────▼────────────┐
              │         LanceDB         │
              │  .forgeplan/lance/      │
              │  (Arrow columnar)       │
              └────────────┬────────────┘
                           │
              ┌────────────▼────────────┐
              │   Markdown Projection    │
              │  .forgeplan/prds/*.md    │
              │  (git-tracked, read-only)│
              └─────────────────────────┘
```

### Migration: existing workspace → LanceDB

```
forgeplan init (new workspace):
  → Create .forgeplan/lance/ + tables
  → Create .forgeplan/prds/, rfcs/, etc. (empty, for projections)

forgeplan init --migrate (existing workspace):
  → Scan .forgeplan/prds/*.md, rfcs/*.md, etc.
  → Parse frontmatter + body from each
  → Insert into LanceDB artifacts table
  → Parse links from frontmatter → insert into relations table
  → Mark migration complete in config.yaml (storage_version: 2)
```

### Directory Structure (updated)

```
.forgeplan/
├── config.yaml          ← storage_version: 2
├── lance/               ← LanceDB (gitignore)
│   ├── artifacts.lance/
│   ├── evidence.lance/
│   └── relations.lance/
├── prds/                ← markdown projections (git-tracked)
├── epics/
├── specs/
├── rfcs/
├── adrs/
├── problems/
├── solutions/
├── evidence/
├── notes/
└── refresh/
```

---

## Risks & Open Questions

- **Risk**: LanceDB crate compile time — arrow + tokio = heavy. Mitigated: workspace dependency dedup.
- **Risk**: LanceDB API stability — SDK not 1.0 yet. Mitigated: pin version, ArtifactStore trait for swapability.
- **Risk**: Binary size increase (arrow + tokio). Mitigated: check NFR-002 (< 15MB) after integration.
- **Open**: embedding column — fill with zeros initially, populate when ONNX added (Phase 4)?
- **Open**: .forgeplan/lance/ в .gitignore? Да — LanceDB files не для git, markdown projections — для git.

## Implementation Phases

### Phase D: LanceDB Integration
- [x] **D.1** Async migration — tokio runtime, async core functions, async tests
- [x] **D.2** LanceDB db module — schema, connect, create tables
- [x] **D.3** ArtifactStore trait + LanceStore implementation (CRUD + convert)
- [ ] **D.4** Markdown projection — write LanceDB → render markdown
- [ ] **D.5** Command migration — all 10 CLI commands use LanceStore

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| PRD-001 | PRD | based_on |
| RFC-001 | RFC | extends |
| RFC-002 | RFC | extends |
| ADR-002 | ADR | implements (LanceDB decision) |
| EPIC-001 | Epic | parent |

---

> **Next step**: D.1 — Async migration (tokio runtime, convert core to async, fix all tests).
