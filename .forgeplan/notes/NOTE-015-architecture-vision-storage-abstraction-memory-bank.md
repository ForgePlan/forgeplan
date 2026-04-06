---
depth: standard
id: NOTE-015
kind: note
links:
- target: ADR-003
  relation: informs
- target: EPIC-002
  relation: informs
status: draft
title: Architecture vision — storage abstraction + memory bank
---

## Storage Abstraction Layer (v2.0 scope)

### Идея
Раз LanceDB = cache layer (ADR-003), можно абстрагировать storage за trait и подключать разные драйверы.

### Trait design
```rust
trait StorageDriver: Send + Sync {
    async fn index_artifact(&self, id: &str, frontmatter: &Map, body: &str);
    async fn search_text(&self, query: &str, limit: usize) -> Vec<SearchHit>;
    async fn search_vector(&self, vector: &[f32], limit: usize) -> Vec<SearchHit>;
    async fn get_relations(&self, id: &str) -> Vec<(String, String)>;  // (target_id, relation_type)
    async fn store_embedding(&self, id: &str, vector: Vec<f32>);
    async fn reindex_all(&self, workspace: &Path);
}
```

### Drivers
| Driver | Use case | Binary size impact |
|--------|----------|-------------------|
| LanceDB | Default embedded, vectors built-in | +100MB (current) |
| SQLite | Lightweight, no vector but fast structured | +2MB |
| PostgreSQL + pgvector | Team/server mode, multi-user | External dep |
| InMemory | Tests, CI, ephemeral | 0 |

### Когда делать
НЕ сейчас. ADR-001 говорит: no adapter traits пока нет второго implementation. Делать когда реально нужен SQLite driver (distribution: binary size).

### Plugin system
Можно сделать drivers как dynamic libraries (.dylib/.so) чтобы не включать все в бинарник. Или feature flags (текущий подход с semantic-search).

---

## Memory Bank (v2.0 scope)

### Идея
Встроенный activity log + decision memory для каждого пользователя. Как Hindsight, но project-specific и git-native.

### Структура
```
.forgeplan/memory/
├── decisions.log    ← автоматически: каждый route, activate, score
├── context.log      ← session start/end, current focus
├── insights.log     ← AI agent записывает находки
└── index.json       ← быстрый lookup по keywords
```

### Что записывается автоматически
- forgeplan route 'X' → depth=Standard → logged
- forgeplan activate PRD-020 → logged  
- forgeplan score PRD-020 → R_eff=1.00 → logged
- AI agent creates artifact → logged with context

### Что записывается по запросу
- forgeplan remember 'chose BGE-M3 because best gap'
- forgeplan remember 'PROB-014 found during smart search test'

### Связь с Hindsight
- Hindsight = personal memory (cross-project, cloud-synced)
- Forgeplan memory = project memory (git-tracked, team-shared)
- Bridge: forgeplan sync-memory → push key decisions to Hindsight

### Driver abstraction
```rust
trait MemoryDriver: Send + Sync {
    async fn log(&self, entry: MemoryEntry);
    async fn recall(&self, query: &str, limit: usize) -> Vec<MemoryEntry>;
    async fn recent(&self, count: usize) -> Vec<MemoryEntry>;
}

// Drivers:
// - FileDriver: append to .log files (default, git-native)
// - HindsightDriver: sync to Hindsight MCP
// - LanceDriver: indexed memory with vector search
```

