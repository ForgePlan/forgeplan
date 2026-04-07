---
depth: standard
id: PRD-042
kind: prd
links:
- target: EPIC-003
  relation: refines
- target: PRD-018
  relation: supersedes
status: draft
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
FR-001  ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  Vector search в knowledge.rs
FR-002  ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  CLI flag --semantic
FR-003  ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  Graceful fallback to keyword
─────────────────────────────────────────────────
TOTAL                              0/3  ( 0%)
```

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


