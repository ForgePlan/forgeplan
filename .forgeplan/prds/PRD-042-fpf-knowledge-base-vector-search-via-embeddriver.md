---
depth: standard
id: PRD-042
kind: prd
links:
- target: EPIC-003
  relation: refines
- target: PRD-018
  relation: supersedes
status: active
title: FPF Knowledge Base — Vector Search via EmbedDriver
---

---
id: PRD-042
title: "FPF Knowledge Base — Vector Search via EmbedDriver"
status: Draft
author: gogocat
created: 2026-04-07
updated: 2026-04-07
priority: P2
depth: standard
parent_epic: EPIC-003
---

# PRD-042: FPF Knowledge Base — Vector Search via EmbedDriver

## Progress

```
FR-001  ████████████████████████  1/1  Vector search (core + CLI + MCP)   ✓ Sprint 13.7
FR-002  ████████████████████████  1/1  CLI flag --semantic                ✓ Sprint 13.7
FR-003  ████████████████████████  1/1  Graceful fallback (compile + runtime) ✓ Sprint 13.7
─────────────────────────────────────────────────
TOTAL                              3/3  (100%) — COMPLETE
```

## Implementation map (FR → file:line → test)

| FR | Surface | Implementation | Tests |
|---|---|---|---|
| FR-001 | `LanceStore::search_fpf_by_vector(query_vec, limit)` | `crates/forgeplan-core/src/db/store.rs::search_fpf_by_vector` — LanceDB native vector_search with Cosine distance | `search_fpf_by_vector_happy_path` (ordering + len), `search_fpf_by_vector_empty_db_returns_empty`, `search_fpf_by_vector_all_null_column_returns_empty` (migration path), `search_fpf_by_vector_nan_query_handled`, `search_fpf_by_vector_inf_query_handled`, `search_fpf_by_vector_off_by_one_dim_errors`, `search_fpf_by_vector_empty_slice_errors`, `search_fpf_by_vector_limit_zero`, `search_fpf_by_vector_limit_max`, `search_fpf_by_vector_unicode_query_chunks` |
| FR-001 | CLI `forgeplan fpf search --semantic` | `crates/forgeplan-cli/src/commands/fpf.rs::run_search` + `try_semantic_search` helper | unit tests for validation + fallback path |
| FR-001 | MCP `forgeplan_fpf_search` with `semantic: Option<bool>` | `crates/forgeplan-mcp/src/server.rs::forgeplan_fpf_search` — typed `FpfSearchResponse` with warning field | `fpf_param_validation_tests` + types.rs serialization tests |
| FR-002 | CLI `--semantic` flag | `crates/forgeplan-cli/src/main.rs::FpfCommands::Search` | parsing tests |
| FR-003 | **compile-time** fallback (feature off) | `#[cfg(not(feature = "semantic-search"))]` warning + keyword path in both CLI and MCP | `run_search_semantic_fallback_warning` unit test + E2E script |
| FR-003 | **runtime** fallback (feature on but Embedder init / encode / vector search fails) | `try_semantic_search` helper with encoder closure in CLI; MCP handler defensive chain | `semantic_fallback_on_embedder_init_fail`, `semantic_fallback_on_encode_fail`, `semantic_fallback_on_search_fail`, `semantic_success_returns_results` |

### Core API additions

- **Schema**: `crates/forgeplan-core/src/db/schema.rs::fpf_spec_schema()` — new `embedding` column `FixedSizeList<Float32, 1024>`, nullable. Rustdoc documents feature-flag contract: column is unconditional, encoding is feature-gated.
- **Migration**: `crates/forgeplan-core/src/db/migrate.rs::migrate_fpf_spec()` — `NewColumnTransform::AllNulls`, idempotent, preserves pre-existing rows. Called from `run_migrations` when fpf_spec table exists.
- **Store**: `LanceStore::insert_fpf_chunks(&[FpfChunk], Option<&[Vec<f32>]>)` — accepts optional embeddings, validates length match + per-vec dim=1024 + **NaN/Inf rejection** at boundaries.
- **Store**: `LanceStore::search_fpf_by_vector(&[f32], usize)` — validates dim=1024, uses LanceDB native `table.vector_search()` with `DistanceType::Cosine`. Graceful Ok(empty) on all-null column (migration path) or LanceDB errors (logged to stderr).
- **Trait**: `FpfStorage::insert_fpf_chunks` signature extended with `Option<&[Vec<f32>]>` — architectural honesty restored (audit Arch H1 fix).
- **CLI helper**: `try_semantic_search<F>(store, query, limit, encoder: F) -> Result<Vec<FpfChunk>>` — closure-based defensive wrapper, testable without BGE-M3.
- **Response type**: `FpfSearchResponse { query, semantic, count, results, warning }` + `FpfSearchHit { id, section_id, title, snippet, line_count }` — typed MCP contract.

**Sprint 13.7 delivered:** All 3 FRs implemented across CLI + MCP (two surfaces), with 5-commit progression (core → CLI → MCP parity → audit fixes → wave 2 completion). 5 commits on `feat/sprint-13.7-prd-042-kb-vector-search`. 1109 tests pass (+34 from baseline). E2E regression 16/16 on release binary. Full /forge-cycle: 4 parallel auditors → fixer → MCP parity agent → completer → manual UX verification by team-lead.

See EVID-064 for full audit + fix detail.

**Supersedes PRD-018** (false-active stub from Sprint 12 deferred item).

---

## Executive Summary

### Vision

FPF Knowledge Base — поиск по 204 секциям FPF spec — получает semantic search **поверх существующего keyword search** через гибридный подход. Закрывает PRD-018 (active stub без реализации) и deferred задачу из Sprint 12.

### Problem

Search логика для FPF KB живёт в `core/db/store.rs::search_fpf` (line 927). Текущая реализация — **per-word OR matching с scoring**: title × 50, body × min(20), all_in_title bonus. Это лучше чем substring grep, но **всё ещё keyword-only**.

При запросах типа "how to evaluate alternatives" пользователь не находит C.2 (F-G-R scoring) — слова "evaluate", "alternatives" отсутствуют в точной форме. Embeddings решают эту проблему семантически.

**Дополнительно:** PRD-018 "FPF Knowledge Base — semantic search" уже существует в active со статусом R_eff=1.0, но содержит только template body. Это **stub artifact** — был активирован без реализации. Этот PRD-042 superseded PRD-018 с правильным shape'ом и реализацией.

**Текущее состояние схемы:**
- `fpf_spec` table в `core/db/schema.rs:104` имеет колонки: id, section_id, parent_section, title, body, line_count, file_path, created_at
- **embedding column отсутствует** — нужна schema migration
- `EmbedDriver` trait существует в `core/driver/mod.rs:205`, но не подключён к FPF KB

**Impact:** FPF KB остаётся keyword-only despite готовая infrastructure. AI-агент через `forgeplan reason --fpf` получает не самые релевантные секции.

### Target Users

| Персона | Боль |
|---------|------|
| AI-агент (forgeplan reason --fpf) | Получает не самые релевантные секции для ADI |
| Разработчик (CLI) | `forgeplan fpf search` промахивается на парафразах |

### Differentiators

- **Использует существующий EmbedDriver trait** (`core/driver/mod.rs:205`) и fastembed wrapper (`core/embed/mod.rs`)
- **Hybrid search**, не replacement — комбинирует existing keyword scoring с semantic
- **Graceful degradation** — если semantic-search feature off, fallback на keyword (как сейчас)
- **Закрывает PRD-018** (false-active stub) через supersede
- **Закрывает deferred Sprint 12 item** (NOTE-039, FPF KB vector search)

---

## Success Criteria

| ID | Criterion | Metric | Target |
|----|-----------|--------|--------|
| SC-1 | Semantic search работает на FPF KB | precision@5 | Семантические запросы возвращают релевантные секции |
| SC-2 | Backward compat сохранена | 0 breaking changes | `forgeplan fpf search "trust"` keyword path работает as before |
| SC-3 | Feature gating | semantic-search feature off → keyword only | `cargo build --no-default-features` works |

---

## Product Scope

### MVP (In-Scope)

- `core/fpf/knowledge.rs` — добавить `search_semantic()` метод используя `EmbedDriver` trait
- `forgeplan fpf search <query> --semantic` — CLI flag для force semantic
- `forgeplan fpf search <query>` (default) — smart: semantic если доступен, иначе keyword
- Embedding cache для FPF sections (build once at ingest, reuse)
- Graceful fallback: если EmbedDriver fails или feature off → keyword search

### Out of Scope

- Hybrid search BM25+semantic (отдельный паттерн, выходит за scope KB)
- Re-embedding при изменении FPF spec (manual `forgeplan fpf ingest --reembed`)
- Multi-language FPF KB (FPF spec на английском, embeddings monolingual)
- Custom embedding models per FPF section type

### Growth Vision

- Hybrid search в knowledge.rs (после PRD-039 BM25 готов — можно reuse)
- Re-ranking на основе section hierarchy (parent sections boost)
- Cross-reference: query → relevant FPF sections → relevant artifacts (граф)

---

## User Journeys

### Journey 1: AI-агент через `forgeplan reason --fpf`

| Шаг | Действие | Ответ |
|-----|----------|-------|
| 1 | `forgeplan reason PRD-039 --fpf` | Reason engine ищет relevant FPF sections для context |
| 2 | Internal call: `knowledge.search_semantic("how to score alternatives")` | Возвращает C.2 (F-G-R), B.5 (Reasoning Cycle) |
| 3 | LLM получает 2-3 relevant FPF sections | ADI reasoning enriched с правильными FPF принципами |

### Journey 2: Разработчик ищет FPF секцию

| Шаг | Действие | Ответ |
|-----|----------|-------|
| 1 | `forgeplan fpf search "compare options"` | Возвращает C.2 (F-G-R), B.3 (Trust), B.5 (Reasoning) |
| 2 | Раньше: keyword "compare" не находил эти секции | — |
| 3 | `forgeplan fpf section C.2` | Открывает найденную секцию |

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Core | Must | [System] can search FPF knowledge base sections by semantic similarity using existing EmbedDriver trait | Journey 1 |
| FR-002 | CLI | Should | [User] can force semantic search with `forgeplan fpf search <query> --semantic` flag | Journey 2 |
| FR-003 | Reliability | Must | [System] gracefully falls back to keyword search when EmbedDriver unavailable or semantic-search feature is disabled | Both |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric |
|----|----------|-------------|--------|
| NFR-001 | Performance | Semantic search shall complete | < 200ms on 204 sections (with cache) |
| NFR-002 | Build size | semantic-search feature off shall produce same binary as before | bytewise diff = 0 |
| NFR-003 | Backward compat | Existing `fpf search` API shall remain | 0 breaking changes |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation |
|----|------|-------------|--------|------------|
| R-1 | EmbedDriver init slow → fpf search lag | Med | Med | Lazy init + cache embeddings on first call |
| R-2 | FPF sections re-ingested → cache invalidation | Low | Low | Cache key = section content hash |
| R-3 | Semantic results хуже keyword на коротких queries | Med | Low | Hybrid (max(keyword, semantic)) — но это в Growth Vision |

---

## Affected Files

- `crates/forgeplan-core/src/db/schema.rs` — add `embedding` column to `fpf_spec_schema()` (FixedSizeList<Float32, 1024>)
- `crates/forgeplan-core/src/db/migrate.rs` — migration v3 → v4 для existing workspaces (re-ingest FPF if missing embeddings)
- `crates/forgeplan-core/src/db/store.rs:927` — extend `search_fpf` with hybrid keyword+vector path (gated by `semantic-search` feature)
- `crates/forgeplan-core/src/fpf/knowledge.rs` — encode embeddings в `ingest_fpf_directory()` when feature enabled
- `crates/forgeplan-core/src/db/store.rs` — `ingest_fpf_chunks()` (existing) — accept optional embedding vectors
- `crates/forgeplan-cli/src/commands/fpf.rs:71` — add `--semantic` flag to `run_search()`
- `crates/forgeplan-core/src/embed/mod.rs` — potentially expose `Embedder::embed_batch()` if not already

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-003 | parent epic | draft |
| PRD-018 | superseded by this (FPF KB semantic search stub) | active (will become superseded) |
| RFC-001 | parent (FPF Engine) | active |
| RFC-003 | foundation (Driver Layer with EmbedDriver) | active |
| NOTE-039 | source idea (deferred Sprint 12) | active |
| sources/RuVector | inspiration (vector search patterns) | external |



