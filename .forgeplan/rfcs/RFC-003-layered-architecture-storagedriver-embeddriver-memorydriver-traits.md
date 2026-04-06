---
depth: standard
id: RFC-003
kind: rfc
links:
- target: ADR-003
  relation: based_on
- target: NOTE-015
  relation: based_on
- target: PROB-014
  relation: informs
- target: EPIC-002
  relation: informs
status: draft
title: Layered Architecture — StorageDriver, EmbedDriver, MemoryDriver traits
---

## Summary

Рефакторинг Forgeplan core на слоистую архитектуру с pluggable backends. Каждый внешний сервис (storage, embeddings, memory, LLM) абстрагирован за trait с реализациями-драйверами. Выбор driver через config.yaml.

## Motivation

1. **Single source of truth (ADR-003)** — ГЛАВНАЯ причина. Сейчас LanceDB = truth, markdown = projection. Два source of truth = sync проблемы (forgeplan update не обновляет .md, rm -rf lance/ = потеря данных). Нужно инвертировать: .md файлы = truth, DB = index. StorageDriver trait отделяет "запись в файлы" от "индексация в DB".
2. **Тесты зависят от LanceDB** — InMemory driver ускорит тесты и уберёт зависимость от диска.
3. **Memory bank не существует** — нужен activity log (decisions, context, insights) с MemoryDriver trait.
4. **Жёсткая привязка к LanceDB** — core напрямую вызывает LanceStore. Невозможно заменить без переписывания.
5. **LLM driver уже работает** (provider switching) — нужно сделать то же для storage/embed/memory.
6. ~~Binary size~~ — решено release profile (163MB→41MB). НЕ причина для абстракции.

## Goals

- G1: Определить 4 trait boundaries (Storage, Embed, Memory, LLM)
- G2: Рефакторнуть LanceStore в impl StorageDriver for LanceDriver
- G3: Добавить InMemory driver для тестов
- G4: Config-driven выбор driver
- G5: Zero breaking changes для CLI/MCP пользователей

## Non-Goals

- НЕ реализовывать SQLite/Postgres driver в этом RFC (только trait + Lance adapter)
- НЕ dynamic loading (.dylib) — feature flags достаточно
- НЕ менять direction of truth (остаётся ADR-003 scope)

## Options Considered

### Option A: Feature flags (compile-time)
Плюс: binary содержит только нужное. Минус: нужна перекомпиляция для смены driver.

### Option B: Enum dispatch (runtime)
Плюс: один бинарник, config выбирает. Минус: все drivers в бинарнике.

### Option C: Гибрид (ВЫБРАНО)
Feature flags для тяжёлых deps (Lance, Postgres), enum dispatch для лёгких (SQLite, InMemory).

## Trade-off Analysis

| Критерий | Option A | Option B | Option C |
|----------|----------|----------|----------|
| Binary size | Best | Worst | Good |
| User flexibility | Bad (recompile) | Best (config) | Good |
| Complexity | Low | Medium | Medium |
| Testing | Good | Good | Best |

## Proposed Direction

### Architecture Layers

```
CLI / MCP / Tauri                          — UI
Core Engine (routing, scoring, validation) — Business Logic
Storage | Embedder | Memory | LLM         — Trait Abstractions
LanceDB | fastembed | FileLog | OpenAI     — Implementations (via config)
```

### Trait Definitions

**StorageDriver**: save/get/list/update/delete artifacts + relations + optional vectors + optional FPF KB + reindex_from_files

**EmbedDriver**: embed/embed_batch/dim/model_name

**MemoryDriver**: log/recall/recent

**LlmDriver**: generate (already implemented as LlmClient)

### MemoryEntry Schema

- timestamp, kind (decision/context/insight/action), content, source, artifact_id, metadata

### Config

```yaml
storage:
  driver: lancedb        # lancedb | sqlite | memory
embedding:
  model: bge-m3          # bge-m3 | embedding-gemma-300m | none
memory:
  driver: file           # file | hindsight | none
llm:
  provider: gemini
  model: gemini-2.5-pro
```

## Risks

- R1: Trait boundary too wide. Mitigation: start minimal, extend.
- R2: dyn dispatch overhead (~2ns). Mitigation: irrelevant for I/O-bound tool.
- R3: Migration complexity. Mitigation: wrap existing code, do not rewrite.

## Open Questions

- Q1: Should petgraph graph be inside StorageDriver or separate GraphDriver?
- Q2: Should R_eff scoring be computed on-the-fly or stored in DB?

## Implementation Phases

### Phase 1: Trait definitions + Lance adapter (v0.12)
- [ ] Define StorageDriver, EmbedDriver, MemoryDriver, LlmDriver traits in driver/ module
- [ ] Wrap LanceStore as impl StorageDriver for LanceDriver
- [ ] Wrap Embedder as impl EmbedDriver for FastEmbedDriver
- [ ] Extract LlmDriver trait from LlmClient
- [ ] Create InMemoryStorage for tests
- [ ] Core Engine accepts dyn StorageDriver instead of LanceStore
- [ ] All tests pass with both Lance and InMemory

### Phase 2: Memory driver + file log (v0.13)
- [ ] FileMemoryDriver: append-only .log files in .forgeplan/memory/
- [ ] Auto-log: route, activate, score, create decisions
- [ ] forgeplan remember CLI command
- [ ] forgeplan recall CLI command

### Phase 3: SQLite driver (v0.14)
- [ ] SQLite schema (artifacts, relations, vectors tables)
- [ ] sqlite-vec extension for vector search
- [ ] Feature flag: default=sqlite, opt-in=lance
- [ ] Binary size target: ~25MB default

### Phase 4: Config-driven selection (v0.14)
- [ ] Parse storage.driver from config.yaml
- [ ] Instantiate correct driver at startup
- [ ] forgeplan init shows available drivers and capabilities

## Affected Files

- crates/forgeplan-core/src/driver/ — NEW module with trait definitions
- crates/forgeplan-core/src/driver/lance.rs — adapter wrapping current LanceStore
- crates/forgeplan-core/src/driver/memory_file.rs — file-based memory
- crates/forgeplan-core/src/driver/in_memory.rs — test driver
- crates/forgeplan-core/src/db/store.rs — refactor to impl StorageDriver
- crates/forgeplan-core/src/embed/mod.rs — refactor to impl EmbedDriver
- crates/forgeplan-core/src/llm/mod.rs — extract LlmDriver trait
- crates/forgeplan-core/src/config/types.rs — storage + memory config sections
- All core modules using LanceStore directly — accept dyn StorageDriver

## Related Artifacts

- ADR-003: Markdown files as source of truth
- NOTE-015: Storage abstraction + memory bank vision
- PROB-014: Smart search integration gaps
- EPIC-002: Forgeplan v2.0 Knowledge OS
