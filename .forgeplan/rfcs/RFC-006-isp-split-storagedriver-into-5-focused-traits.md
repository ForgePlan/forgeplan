---
depth: standard
id: RFC-006
kind: rfc
links:
- target: PROB-015
  relation: based_on
status: active
title: ISP Split — StorageDriver into 5 focused traits
---

# RFC-006: ISP Split — StorageDriver into 5 focused traits

## Summary

Split the 29-method `StorageDriver` god-trait into 5 focused traits following the Interface Segregation Principle. This unblocks SQLite driver (Sprint 5) and eliminates dead trait code.

## Problem

`StorageDriver` has 29 methods across 7 conceptual domains (CRUD, relations, search, vectors, FPF, lifecycle, scoring). Every new backend must implement ALL 29 methods even if it only needs CRUD. PROB-015 H4 identified this as an ISP violation. Dead traits (`MemoryDriver`, `LlmDriver`) add confusion.

## Goals

- Split StorageDriver into focused traits with clear bounded contexts
- Keep `StorageDriver` as supertrait for backward compatibility during transition
- Make VectorStorage and FpfStorage optional (not required for SQLite)
- Remove dead `MemoryDriver` and `LlmDriver` traits
- Zero breaking changes to 712+ tests that use `&LanceStore` directly

## Non-Goals

- Not migrating consumers from `&LanceStore` to `&dyn ArtifactStorage` (future work)
- Not adding SQLite driver (Sprint 5)
- Not changing `EmbedDriver` `&mut self` to `&self` (separate concern)

## Target Users

- [Developer] implementing new storage backends (SQLite, PostgreSQL)
- [AI Agent] using forgeplan MCP tools with storage abstraction

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-015 | based_on |
| RFC-003 | based_on |

## Design

### Trait Hierarchy

```rust
// Core traits (REQUIRED for any backend)
pub trait ArtifactStorage: Send + Sync {
    // 9 methods: create, get, get_record, list, list_records,
    //            update, update_body, update_r_eff, delete
}

pub trait RelationStorage: Send + Sync {
    // 5 methods: add, delete, get, get_incoming, get_all
}

pub trait SearchStorage: Send + Sync {
    // 3 methods: search_body, find_stale, next_id
}

// Optional traits (NOT required for basic backends)
pub trait VectorStorage: Send + Sync {
    fn supports_vectors(&self) -> bool { false }
    // 2 methods: vector_search, update_embedding (with defaults)
}

pub trait FpfStorage: Send + Sync {
    fn has_fpf(&self) -> bool { false }
    // 5 methods: insert, search, get_section, list_sections, clear (with defaults)
}

// Supertrait — backward compatible
pub trait StorageDriver: ArtifactStorage + RelationStorage + SearchStorage + VectorStorage + FpfStorage {}

// Blanket impl — any type that implements all 5 traits IS a StorageDriver
impl<T> StorageDriver for T
where T: ArtifactStorage + RelationStorage + SearchStorage + VectorStorage + FpfStorage {}
```

### Lifecycle methods (open, init)

Move to factory module. They use `Self: Sized` which prevents `dyn StorageDriver` anyway.

### Dead traits

Delete `MemoryDriver` (3 methods) and `LlmDriver` (1 method) — never implemented by any backend.

## Implementation Phases

### Phase 1: Trait Definition (15 min)
- [ ] Define 5 sub-traits in `driver/mod.rs`
- [ ] Add blanket `StorageDriver` impl
- [ ] Remove `MemoryDriver`, `LlmDriver` dead traits
- [ ] Move `open`/`init` out of trait (to factory or associated functions)
- [ ] `cargo check` — 0 errors

### Phase 2: Implementations (30 min)
- [ ] `InMemoryStore` — split `impl StorageDriver` into 5 `impl` blocks
- [ ] `LanceDriver` — split `impl StorageDriver` into 5 `impl` blocks
- [ ] `factory.rs` — update return types
- [ ] `driver_test.rs` — update 8 tests to use sub-traits
- [ ] `cargo test` — ALL pass

### Phase 3: EmbedDriver + Cleanup (15 min)
- [ ] Verify `EmbedDriver` trait (already defined)
- [ ] Add `NoOpEmbedDriver` (returns empty vec)
- [ ] Clean up factory code duplication (PROB-015 M3)
- [ ] `cargo test` — ALL pass
- [ ] Evidence + PR

## Acceptance Criteria

1. `StorageDriver` trait has 0 direct methods (supertrait only)
2. 5 focused traits with clear single responsibility
3. `VectorStorage` and `FpfStorage` have default implementations (optional)
4. Dead traits removed
5. All 730+ tests pass unchanged
6. `dyn StorageDriver` still works (backward compatible)


