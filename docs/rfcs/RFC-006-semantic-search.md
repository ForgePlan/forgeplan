---
id: RFC-006
title: "Semantic Search — fastembed + LanceDB vector index"
status: Accepted
author: explosovebit
created: 2026-03-22
updated: 2026-03-22
prd: PRD-001
depth: deep
---

# RFC-006: Semantic Search

## Progress

```
Phase 4F  ████████████████████████  4/4   (100%)  Semantic Search  ✅ DONE
─────────────────────────────────────────────────
TOTAL                               4/4   (100%)
```

## Summary

Добавить vector embeddings и семантический поиск в Forgeplan. Артефакты эмбеддятся при создании/обновлении. `forgeplan search --semantic "query"` ищет по смыслу, а не по substring.

## Motivation

Текущий `forgeplan search` = substring match. Бесполезен для:
- "найди решения похожие на это" (семантическое сходство)
- "какие артефакты связаны с аутентификацией" (синонимы, парафразы)
- "найди PRD по описанию проблемы" (cross-artifact reasoning)

## Architecture

```
fastembed-rs (BGE-M3, 1024-dim)
    ↓ embed text → Vec<f32>
LanceDB artifacts table
    column: embedding FixedSizeList(1024, Float32)
    ↓ ANN vector search
forgeplan search --semantic "query"
```

Per ADR-005: fastembed-rs behind `semantic-search` Cargo feature flag.

## Implementation Phases

- [x] **F.1** `embed/` module — fastembed wrapper behind feature flag
- [x] **F.2** LanceStore::update_embedding() + vector_search() methods
- [x] **F.3** `forgeplan search --semantic` — vector search via LanceDB
- [x] **F.4** RFC-006 documented, feature flag `semantic-search` in Cargo.toml

## F.1: embed/ module

```rust
// Behind #[cfg(feature = "semantic-search")]
pub struct Embedder {
    model: fastembed::TextEmbedding,
}

impl Embedder {
    pub fn new() -> Result<Self> { ... }  // lazy init, auto-download model
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> { ... }
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> { ... }
}
```

## F.2: Embed on create/update

LanceDB artifacts table already has `embedding` column (FixedSizeList(1024, Float32), nullable).

On `forgeplan new` / `forgeplan generate` / `forgeplan update --body`:
1. If `semantic-search` feature enabled: embed body → store in `embedding` column
2. If not: leave `embedding` as null

## F.3: Vector search

```
forgeplan search "authentication" --semantic
```

1. Embed query text → query_vec
2. `LanceDB table.search(query_vec).limit(10).execute()`
3. Return results sorted by distance

## F.4: MCP tool

`forgeplan_semantic_search { query, limit? }` — returns closest artifacts.

## Feature Flag

```toml
# Cargo.toml
[features]
default = []
semantic-search = ["fastembed"]

[dependencies]
fastembed = { version = "5", optional = true }
```

Without flag: `forgeplan search` = substring only, binary < 15MB.
With flag: `forgeplan search --semantic` available, binary ~40MB.

## References

- ADR-005: fastembed-rs + BGE-M3 (1024-dim)
- RFC-003: LanceDB integration (embedding column in schema)
