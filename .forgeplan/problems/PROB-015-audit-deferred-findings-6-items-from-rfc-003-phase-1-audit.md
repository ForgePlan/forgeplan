---
depth: standard
id: PROB-015
kind: problem
links:
- target: RFC-003
  relation: informs
- target: PROB-014
  relation: informs
status: deprecated
title: Audit deferred findings — 6 items from RFC-003 Phase 1 audit
---

## Signal

6 audit findings отложены из RFC-003 Phase 1 audit (2026-03-30). Не критичны для текущего кода, но станут проблемой при Phase 2+.

## Context

Audit: 3 агента (Rust safety, Logic+SOLID, Test coverage). 13 findings total. 7 fixed. 6 deferred.

## Deferred Findings

### HIGH — Phase 2

**H2. EmbedDriver &mut self blocks async executor**
- File: driver/mod.rs:174-186
- Problem: `&mut self` в `embed()` означает нельзя вызвать из нескольких async tasks без Mutex
- Fix: сменить на `&self` + interior mutability, или сделать embed() async + spawn_blocking
- When: Phase 2 когда EmbedDriver будет использоваться в smart search

**H4. StorageDriver = 29-method God Interface (ISP violation)**
- File: driver/mod.rs:23-171
- Problem: любой новый backend должен реализовать все 29 методов
- Fix: split на ArtifactStore + RelationStore + SearchStore + VectorStore + FpfStore
- When: Phase 2 перед добавлением SQLite driver
- Note: open/init на trait с Self:Sized = dead weight, убрать в пользу factory

### MEDIUM — Phase 2

**M3. Factory code duplication**
- File: driver/factory.rs:13-52
- Problem: create_storage и init_storage почти идентичны
- Fix: extract helper с enum mode parameter

**M4. Filter logic duplication**
- File: driver/in_memory.rs
- Problem: list_artifacts и list_records имеют copy-paste filter closure
- Fix: extract fn matches_filter()

**M5. MemoryDriver + LlmDriver = dead traits**
- File: driver/mod.rs:189-206
- Problem: определены но нет ни одной реализации
- Fix: реализовать FileMemoryDriver в Phase 2, или убрать если не нужны
- Note: MemoryConfig в config.yaml уже есть, driver field = file, но driver не существует

### TEST GAPS — Phase 2

**T1. vector_search round-trip (P0)**
- update_embedding → vector_search = ZERO tests
- LanceDriver reports supports_vectors()=true но round-trip не проверен

**T2. Edge cases (P1)**
- create_artifact с duplicate ID (InMemory overwrites, Lance appends — behavior divergence)
- add_relation с nonexistent artifact IDs (silently creates dangling relation)
- update_artifact(None, None) no-op case
- search_body с empty query
- find_stale на Lance backend (only InMemory tested)
- list_records + update_body на Lance backend

## Impact

Не блокирует текущий функционал. Станет проблемой при:
- Phase 2: Memory driver (M5 — dead trait)
- Phase 2: Smart search (H2 — EmbedDriver, T1 — vector_search)
- Phase 3: SQLite driver (H4 — God Interface, нужен split)

## Anti-Goodhart

- Не фиксить H4 (ISP) ради ISP — фиксить когда реально мешает добавить 2й backend
- Не добавлять мёртвые тесты (T2) ради coverage числа — добавлять когда функционал используется

---

## Decision: What we fix NOW (Phase 2 prep) vs DEFER

Router said Standard + PRD. We override: PROB already documents the problem,
adding Goals/Criteria here instead of separate PRD (Variant C).

### Goals (Updated 2026-04-03 — Sprint 4)

1. **H4: ISP split** — 29-method StorageDriver → 5 focused traits (RFC-006)
2. **M5: Dead traits** — remove MemoryDriver, LlmDriver (never implemented)
3. **M3: Factory cleanup** — dedup create_storage/init_storage
4. **H2: EmbedDriver** — verify trait, add NoOpEmbedDriver fallback

### Non-Goals

- M4 (filter dedup in InMemory) — cosmetic, not blocking
- T1/T2 (test gaps) — add when functionality is actually used
- Consumer migration (`&LanceStore` → `&dyn ArtifactStorage`) — future work

### Acceptance Criteria

- [ ] StorageDriver = supertrait with 0 direct methods
- [ ] 5 focused traits: ArtifactStorage, RelationStorage, SearchStorage, VectorStorage, FpfStorage
- [ ] VectorStorage + FpfStorage have default impls (optional for backends)
- [ ] Dead MemoryDriver + LlmDriver removed
- [ ] Factory dedup resolved
- [ ] All 730+ tests pass
- [ ] Evidence created + linked to PROB-015 + RFC-006



