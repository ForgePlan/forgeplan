---
depth: standard
id: PRD-039
kind: prd
links:
- target: PRD-040
  relation: refines
status: draft
title: Smart Search v2 — BM25, Composable Filters, Graph Expansion
---

---
id: PRD-039
title: "Smart Search v2 — BM25, Composable Filters, Graph Expansion"
status: Draft
author: gogocat
created: 2026-04-07
updated: 2026-04-07
priority: P1
depth: standard
domain: general
projectType: cli_tool
---

# PRD-039: Smart Search v2 — BM25, Composable Filters, Graph Expansion

## Progress

```
FR-001   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  (  0%)  BM25
FR-002   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  (  0%)  Filters
FR-003   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  (  0%)  Graph expansion
─────────────────────────────────────────────────
TOTAL                               0/3  (  0%)
```

---

## Executive Summary

### Vision

Smart Search v2 заменяет примитивный substring grep на полноценный BM25 relevance scoring, добавляет composable фильтры и graph-expanded результаты — превращая поиск артефактов из "найти по подстроке" в "найти самое релевантное с контекстом".

### Problem

Текущий keyword scoring в `search/smart.rs` работает бинарно: title exact = 1.0, title contains = 0.8, body contains = 0.5, else 0.0. Это означает:
- PRD с "auth" 20 раз в body и Note с "auth" 1 раз получают одинаковый score (0.5)
- Нет term frequency / document frequency учёта
- Фильтрация ограничена двумя полями (kind, status) — нет фильтра по depth, наличию evidence, дате
- Результаты поиска не показывают связанные артефакты из графа зависимостей

**Impact**: при 20+ артефактах пользователь получает шумные результаты и вынужден вручную проходить по графу чтобы найти связанный контекст.

### Target Users

| Персона | Описание | Ключевая боль |
|---------|----------|---------------|
| AI-агент (MCP) | Claude Code через forgeplan serve | Получает нерелевантные результаты, тратит tokens на фильтрацию |
| Разработчик (CLI) | Человек через forgeplan search | Не видит связанных артефактов, вручную ходит по link/tree |

### Differentiators

- BM25 — стандарт информационного поиска (Elasticsearch, Lucene), но встроенный в ~120 LOC без deps
- Graph expansion — уникальная фича: ни одна vector DB не включает связанные ноды из DAG артефактов в search results
- Паттерны портированы из RuVector (sources/RuVector) с адаптацией к domain model Forgeplan

---

## Success Criteria

| ID | Criterion | Metric | Current | Target | Timeframe | How to Measure |
|----|-----------|--------|---------|--------|-----------|----------------|
| SC-1 | BM25 различает частоту терминов | Precision@5 | binary 0.5 | frequency-weighted | Sprint 14 | unit test: doc с 20x "auth" ranks > doc с 1x |
| SC-2 | Фильтры поддерживают 5+ полей | Filter fields | 2 (kind, status) | 5+ (kind, status, depth, evidence, date) | Sprint 14 | cargo test |
| SC-3 | Graph expansion включает связанные артефакты | Related in results | 0 | depth=1 neighbors | Sprint 14 | search "X" показывает linked artifacts |

---

## Product Scope

### MVP (In-Scope)

- BM25 scoring заменяет substring grep в keyword path Smart Search
- ArtifactFilter enum с composable And/Or логикой
- 1-hop graph expansion в search results (linked artifacts с decay score)
- MinMax score normalization для BM25 scores

### Out of Scope

- SONA self-learning (нет данных для обучения на 20-100 артефактах)
- Hyperedge / N-ary relations (нет use case)
- TF-IDF persistent index (BM25 строится on-the-fly из LanceDB)
- Full-text search с морфологией / стеммингом

### Growth Vision

- Стоп-слова для русского языка (наши PRD на русском)
- BM25 persistent index для масштаба 1000+ артефактов
- 2-hop graph expansion с configurable depth

---

## User Journeys

### Journey 1: AI-агент ищет контекст для reasoning

**Цель пользователя**: найти все релевантные артефакты по теме перед `forgeplan reason`

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan search "authentication" --type prd --json` | Список PRD отсортированных по BM25 relevance | Ранее: бинарный score |
| 2 | Видит PRD-005 (score 0.92) + related: RFC-003, ADR-002 | Graph expansion показывает linked artifacts | Ранее: только direct matches |
| 3 | `forgeplan context PRD-005` | Полный контекст с graph + validation + scoring | Без изменений |

**Результат**: агент получает полный контекст за 1 запрос вместо 3 (search → tree → get).

### Journey 2: Разработчик фильтрует артефакты по нескольким критериям

**Цель пользователя**: найти active PRD с evidence по теме

| Шаг | Действие пользователя | Ответ системы | Заметки |
|-----|----------------------|---------------|---------|
| 1 | `forgeplan search "scoring" --status active --with-evidence` | Только артефакты с R_eff > 0 | Новый фильтр |
| 2 | `forgeplan search "lifecycle" --depth deep` | Только deep артефакты | Новый фильтр |

**Результат**: точный поиск без ручной фильтрации.

---

## Functional Requirements

| ID | Category | Priority | Requirement | Journey |
|----|----------|----------|-------------|---------|
| FR-001 | Search | Must | [User] can search artifacts with BM25 relevance scoring that ranks documents higher when they contain query terms more frequently, replacing binary substring matching | Journey 1 |
| FR-002 | Search | Must | [User] can filter search results using composable criteria: kind, status, depth, has-evidence (R_eff > 0), created-after date, with AND/OR composition | Journey 2 |
| FR-003 | Search | Must | [User] can see 1-hop linked artifacts (neighbors in dependency graph) in search results with decayed relevance score | Journey 1 |

---

## Non-Functional Requirements

| ID | Category | Requirement | Metric | Condition | Measurement |
|----|----------|-------------|--------|-----------|-------------|
| NFR-001 | Performance | BM25 scoring shall complete | < 50ms | On 200 artifacts with 10KB avg body | cargo bench |
| NFR-002 | Binary size | No new dependencies shall be added | 0 new deps | Cargo.toml unchanged | diff Cargo.toml |
| NFR-003 | Compatibility | Existing search API shall remain backward-compatible | 0 breaking changes | CLI + MCP | E2E tests |

---

## Risks & Mitigations

| ID | Risk | Probability | Impact | Mitigation | Owner |
|----|------|-------------|--------|------------|-------|
| R-1 | BM25 on-the-fly too slow at 500+ artifacts | Low | Medium | Add persistent inverted index later (Growth Vision) | dev |
| R-2 | Graph expansion pollutes results with noise | Medium | Low | Decay factor 0.7^depth + max_depth=1 default | dev |

---

## Affected Files

- `crates/forgeplan-core/src/search/smart.rs` — replace keyword_score, add graph expansion
- `crates/forgeplan-core/src/search/mod.rs` — add bm25 module
- `crates/forgeplan-core/src/search/bm25.rs` — NEW: BM25 implementation
- `crates/forgeplan-core/src/search/filter.rs` — NEW: ArtifactFilter enum
- `crates/forgeplan-core/src/db/store.rs` — extend ArtifactFilter struct or replace
- `crates/forgeplan-cli/src/commands/search.rs` — add new CLI flags
- `crates/forgeplan-mcp/src/server.rs` — pass extended filters to search

## Related Artifacts

| Artifact | Relation | Status |
|----------|----------|--------|
| PRD-040 | Sibling (Scoring Intelligence) | Draft |
| sources/RuVector | Pattern source (BM25, Filter DSL) | External |

