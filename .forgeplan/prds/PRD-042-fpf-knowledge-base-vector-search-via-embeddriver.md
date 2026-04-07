---
depth: standard
id: PRD-042
kind: prd
links:
- target: EPIC-003
  relation: refines
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

FPF Knowledge Base (`core/fpf/knowledge.rs`) — поиск по 204 секциям FPF spec — получает semantic search через существующий `EmbedDriver` trait. Закрытие deferred задачи из Sprint 12.

### Problem

`forgeplan fpf search "trust"` сейчас работает только по keyword (substring match). При запросах вроде "how to evaluate alternatives" пользователь не находит секцию C.2 (F-G-R scoring), потому что точного совпадения слов нет.

EmbedDriver уже существует (RFC-003 Phase 2, Sprint 4), используется для embeddings артефактов. Но **knowledge.rs его не использует** — это deferred задача с Sprint 12 (см. SPRINTS.md, P0 list).

**Impact:** FPF KB остаётся keyword-only despite готовая инфраструктура. Семантические запросы не работают, AI-агент не может полноценно использовать FPF context.

### Target Users

| Персона | Боль |
|---------|------|
| AI-агент (forgeplan reason --fpf) | Получает не самые релевантные секции для ADI |
| Разработчик (CLI) | `forgeplan fpf search` промахивается на парафразах |

### Differentiators

- **Использует существующий EmbedDriver** — 0 новых deps, 0 нового embedding кода
- **Graceful degradation** — если semantic-search feature off, fallback на keyword (как сейчас)
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

- `crates/forgeplan-core/src/fpf/knowledge.rs` — add `search_semantic()` method
- `crates/forgeplan-core/src/fpf/mod.rs` — export new method
- `crates/forgeplan-cli/src/commands/fpf.rs` — add `--semantic` flag to search subcommand
- `crates/forgeplan-core/src/embed/mod.rs` — possibly expose helper for KB embedding (if needed)

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| EPIC-003 | parent epic | draft |
| RFC-001 | parent (FPF Engine) | active |
| RFC-003 | foundation (Driver Layer with EmbedDriver) | active |
| NOTE-039 | source idea (deferred Sprint 12) | active |
| sources/RuVector | inspiration (vector search patterns) | external |

