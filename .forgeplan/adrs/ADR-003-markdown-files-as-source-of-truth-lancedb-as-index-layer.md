---
depth: standard
id: ADR-003
kind: adr
links:
- target: PROB-014
  relation: informs
status: active
title: Markdown files as source of truth — LanceDB as index layer
---

## Context

Текущая архитектура: LanceDB = source of truth, markdown = projection (генерируется при forgeplan new).

Проблемы:
- Markdown и LanceDB рассинхронизируются (forgeplan update не обновляет .md)
- rm -rf .forgeplan/lance = потеря данных (если нет export)
- Schema migration LanceDB = боль (нельзя добавить column)
- AI агент не может читать LanceDB напрямую (только через MCP)
- forgeplan init + reinit = рискованная операция

## Decision

**Инвертировать direction of truth**: Markdown файлы = единственный source of truth. LanceDB = index/cache layer.

### Новый поток данных:

```
[User/Agent] → edit .md файл → [Watcher] → parse → [LanceDB + vectors + petgraph]
                                                          ↑
[User/Agent] ← search/query/graph ← [Query Layer] ←──────┘
```

### Что хранится где:

| Данные | Где | Формат |
|--------|-----|--------|
| Артефакт content | .md файл (frontmatter + body) | YAML + Markdown |
| Links/relations | frontmatter related: field | YAML array |
| R_eff score | Computed on-the-fly из evidence files | Не хранится |
| Embeddings | LanceDB vector column (cache) | f32 array |
| Graph | petgraph (in-memory, built from relations) | DiGraph |

### Миграция:

1. forgeplan new → пишет .md файл (уже делает)
2. Background watcher (notify crate) → парсит changes → обновляет index
3. forgeplan reindex → одноразовая full re-sync
4. rm -rf lance/ → не страшно, reindex восстановит всё из .md файлов

## Alternatives Considered

**A. Оставить LanceDB as source of truth** — текущее, работает, но sync проблема растёт.
**B. Полностью убрать LanceDB** — файлы + petgraph only. Потеряем vector search и structured queries.
**C. Markdown = truth, LanceDB = index (ВЫБРАНО)** — лучшее из обоих: git-native files + fast queries.

## Consequences

### Positive
- Нет sync проблемы (одна правда)
- Git-native (diff, merge, review, history)
- AI читает .md напрямую (без MCP для read)
- Нет data loss при rm -rf lance/
- Нет schema migration (frontmatter = flexible)

### Negative
- Нужен watcher daemon или manual reindex
- Parse frontmatter = медленнее чем DB read (но <100ms для 82 artifacts)
- Vector embeddings нужно пересчитывать при изменении body

### Risks
- Watcher может пропустить изменения (mitigation: forgeplan reindex)
- Concurrent writes в .md файлы (mitigation: git handles this)

## Scope

v0.13+ (Deep рефакторинг). Для v0.12 — P0 фиксы из PROB-014 без изменения direction of truth.

## Affected Files

- crates/forgeplan-core/src/db/store.rs — invert write direction
- crates/forgeplan-core/src/workspace/ — watcher (notify crate)
- crates/forgeplan-core/src/embed/ — persist embeddings
- crates/forgeplan-cli/src/commands/ — reindex command


