---
id: ADR-002
title: "LanceDB вместо SQLite для storage"
status: Accepted
depth: standard
valid_until: 2027-03-21
problem_ref: ""
created: 2026-03-21
updated: 2026-03-21
---

# ADR-002: LanceDB вместо SQLite для storage

## Progress

```
Phase 0  ░░░░░░░░░░░░░░░░░░░░░░░░  0/0  (  0%)
─────────────────────────────────────────────────
TOTAL                               0/0  (  0%)
```

---

## Context

Нужно хранилище для артефактов с поддержкой structured queries + vector search. Quint-code использует SQLite (9 таблиц, schema.sql). Forgeplan требует дополнительно semantic search по embedding'ам для поиска связанных артефактов, дубликатов и контекстных рекомендаций.

## Decision

Выбран **LanceDB** -- embedded DB с tables + vectors в одной базе. Дополнительно **Tantivy** для full-text search.

**Selected**: LanceDB + Tantivy

**Why Selected**: Единое embedded хранилище для structured data и vector embeddings, zero-config deployment, Apache Arrow формат для эффективного хранения.

## Alternatives Considered

| Option | Verdict | Why |
|--------|---------|-----|
| SQLite + отдельный vector DB (Qdrant/Chroma) | Rejected | Proven pattern, но два storage engine, сложнее deployment. Overengineering для embedded tool |
| SQLite only (без vectors) | Rejected | Простейший вариант, quint-code proven. Но нет semantic search -- core feature потеряна |
| File-based (markdown only) | Rejected | Zero dependencies, но нет structured queries, search = grep |
| LanceDB + Tantivy | **Chosen** | One embedded DB для structured + vectors, Tantivy для full-text |

## Consequences

### Positive
- Одна embedded DB для structured data и vector embeddings
- Apache Arrow формат -- эффективное columnar хранение
- Zero-config: не нужен отдельный сервер или процесс
- Vector search built-in -- ANN (approximate nearest neighbor) из коробки

### Negative (trade-offs)
- LanceDB Rust SDK менее зрелый чем rusqlite
- Меньше community и примеров по сравнению с SQLite
- Tantivy добавляет вторую зависимость для full-text search

### Risks
- Breaking API changes в LanceDB (SDK ещё не стабилизирован на 1.0)
- Производительность на больших объёмах (>10k артефактов) не проверена

<!-- Depth: standard+ — обязательно для standard, deep, critical -->

## Invariants

- LanceDB = source of truth (structured + vectors)
- Markdown = git-tracked projections (read-only view)
- Sync on every write: write to LanceDB -> render markdown -> save to `.forgeplan/`
- Никогда не читать из markdown для бизнес-логики -- только из LanceDB

## Evidence Requirements

- LanceDB Rust crate компилируется и работает на macOS/Linux
- Write + read roundtrip < 10ms для одного артефакта
- Vector search < 500ms на 1000 артефактах
- Размер базы < 50MB для 1000 артефактов с embeddings

## Valid Until

**Дата**: 2027-03-21

**Обоснование TTL**: 1 год -- к этому моменту Phase 3 будет завершена и будут реальные данные о производительности LanceDB в продакшене.

**Refresh Triggers** (когда пере-оценить досрочно):
- LanceDB Rust SDK прекратит поддержку или заморозит development
- Roundtrip время превысит 100ms на типичных операциях
- SQLite получит встроенную поддержку vector search (sqlite-vec стабилизируется)

## AI Guidance

> Правила для AI-агентов при работе с этим решением.

- Все операции чтения/записи артефактов идут через LanceDB, не через файловую систему
- Markdown файлы генерируются автоматически и не редактируются напрямую
- При генерации кода для storage использовать lancedb crate, не rusqlite
- Если задача конфликтует с этим ADR, явно указать на конфликт

## Implementation Plan

### Phase 0: Foundation
- [ ] **0.1** Добавить lancedb и tantivy в Cargo.toml зависимости
- [ ] **0.2** Определить LanceDB схему таблиц (адаптация из quint-code schema.sql)

### Phase 1: Core
- [ ] **1.1** Реализовать CRUD операции для артефактов через LanceDB
- [ ] **1.2** Реализовать markdown projection renderer

## Implementation Log

<!-- Add wave entries as sprints are completed:

### Wave 1 — YYYY-MM-DD
| Task | Teammate | Status | Files |
|------|----------|--------|-------|
| 0.1 | ... | Done | ... |
-->

## Related Artifacts

| Artifact | Type | Relation |
|----------|------|----------|
| EPIC-001 | Epic | based_on |
| PRD-001 | PRD | informs |
| ADR-001 | ADR | related |
