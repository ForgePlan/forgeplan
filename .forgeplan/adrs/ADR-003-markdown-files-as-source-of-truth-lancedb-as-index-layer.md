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

---

## Amendment 1 (2026-05-01) — Derived-data and sync-mechanism exemptions

After PRD-073 Phase 1–3a established the file-first invariant on mutating
`LanceStore` methods (regression guard `crates/forgeplan-cli/tests/adr_003_invariant.rs`,
EVID-094), two narrow classes of direct `LanceStore::*` calls remain
intentional and are explicitly **out of scope** for the bypass-elimination
ratchet.

### Class A — Derived data with no markdown projection

`LanceStore::update_embedding` and `LanceStore::update_r_eff_score` mutate
columns that are **derived** from artifact content and live in LanceDB as
a cache. The file is not the source of truth for these specific columns
because:

- the embedding vector is a deterministic function of `(model, body)` —
  reproducing it from a hand-edited vector is meaningless and would not
  parse as YAML,
- the R_eff score is a function of `(body, linked evidence, valid_until,
  decay)` — pinning a number into the markdown body would shadow the
  formula, not record it.

The markdown body remains authoritative for the **inputs** that feed
these computations. **Acceptable staleness**: between reindex cycles, the
cached embedding / score may lag behind hand-edited body content — agents
relying on them must re-run `forgeplan reindex` (or call `update_embedding /
update_r_eff_score` again themselves) before trusting the cached value
after a body edit.

This is a real failure mode: `score.rs` reads `record.body` from LanceDB
when computing the score it then persists; if the user edited the markdown
without `reindex`, LanceDB's body is stale → the persisted score is wrong
until the next reindex catches up. Mitigation in scoping callers is
**caller responsibility**, not enforced by the helper layer (audit 2026-05-01).

Therefore: these methods may be called directly from any caller (embedding
pipelines, FPF scoring, batch jobs) **provided the caller is responsible
for ensuring its inputs are fresh**. They are excluded from the
`FORBIDDEN_METHODS` list in the regression guard test.

**Visibility plan (Phase 4)**: when `LanceStore::create_artifact / update_* /
delete_* / add_relation / delete_relation` become `pub(crate)`,
`update_embedding` and `update_r_eff_score` stay `pub`. Phase 4 will also
introduce a `DerivedDataWriter` trait in `forgeplan-core::scoring` and
`forgeplan-core::embed` so direct call sites in `commands/` / `server.rs`
become reviewable as a tight surface, not an open-ended exception.

### Class B — Sync mechanisms (the projection rebuild flow)

The CLI commands `reindex`, `git-sync`, `import_cmd`, `watch`, `ingest`
and the MCP tool `forgeplan_import` ARE the file→store synchronization
flow. They read from a file (or import bundle) and write to LanceDB. For
them, a direct `store.create_artifact / delete_artifact / add_relation`
call **is** the projection-rebuild step — there is no projection to render
because the source side IS the file (or bundle). Routing them through the
"sync_before_mutation → mutate → render_after_mutation" helper would
either no-op (file is already authoritative) or paradoxically overwrite
the input the command was supposed to ingest.

These call sites remain in the regression guard's count today (CLI = 14,
MCP = 3 as of 2026-05-01) and the ratchet stops at those baselines.
PRD-073 **Phase 3b** will extract higher-level helpers — provisional
names: `import_artifact_with_projection`, `reindex_workspace_via_projection`,
`git_sync_workspace_via_projection` — and migrate the remaining sites onto
them. Once both baselines reach 0, **Phase 4** demotes the mutating
`LanceStore` methods to `pub(crate)`. After that, any direct mutation from
`commands/*.rs` or `server.rs` becomes a compile-time error and the
regression test is kept as belt-and-suspenders.

### Status of the migration (2026-05-01, post-audit, post-3b/4 lockdown)

| Phase | Scope | Status |
|---|---|---|
| 1 | Helper API design + canary migration | ✅ done (commit on dev) |
| 2 | MCP lifecycle handlers + bidirectional links | ✅ done (PR #227) |
| 3a | Bulk migration of all visible bypasses + audit remediation | ✅ done (EVID-094): 17 CLI + 7 MCP nominal bypass sites migrated through 9 helpers; multi-line ratchet fixed; atomic writes, prefix-collision, --depth+--title ordering, warn-and-continue semantics, file-first body ordering all corrected per A-AUDIT findings |
| 3b | Sync-mechanism extraction (reindex / git_sync / import_cmd / watch / ingest / forgeplan_import) → higher-level helpers + migration | ✅ done (commit 598c90b): 6 new `sync_*_from_file` helpers + `delete_orphan_*` extracted; reindex/git_sync/watch/ingest/import_cmd migrated; ratchet baselines CLI 17 → 0, MCP 4 → 0 |
| 4 | `pub(crate)` visibility lockdown for `LanceStore::create_artifact / update_artifact / update_body / update_depth / add_tags / remove_tags / delete_artifact / add_relation / delete_relation / delete_relations_for_artifact` | ✅ done (commit 598c90b): all 11 mutating methods demoted; `update_embedding` + `update_r_eff_score` stay `pub` per Class A above; test fixtures use `*_for_test` escape hatches gated on `cfg(any(test, all(feature = "test-helpers", debug_assertions)))` so release builds with the feature accidentally enabled still get the lockdown |
| 5 | Closure EVID with `git clone → reindex → diff` reproducibility (CL3 evidence pack) | ⏳ partial — EVID-094 covers Phase 3a surfaces at CL2; full clone-reproducibility EVID at CL3 deferred to PRD-073 Phase 3c (typed `MutationError` + per-helper migration also lands there) |

This amendment does not change the original decision (markdown is the
source of truth, LanceDB is the derived index). It clarifies which method
families are part of the invariant and which are deliberate carve-outs so
contributors can read the regression guard's `FORBIDDEN_PATTERNS` and
understand both what it covers and what it intentionally does not.

## Amendment 2 — Phase 3c Typed Errors (2026-05-02)

Phase 3a/3b established the file-first invariant by extracting helpers in
`forgeplan_core::projection::*` and demoting `LanceStore::*` mutating
methods to `pub(crate)`. All 16 helpers returned `anyhow::Result<T>`.
Phase 3c migrates them to `MutationResult<T>` (alias for
`std::result::Result<T, MutationError>`) so callers can react per-variant
instead of string-matching on a flattened error message.

### Motivation

`anyhow::Result<()>` collapses every failure mode into a single opaque
chain. Two concrete consequences observed during Phase 3a/3b adversarial
audits:

1. **MCP cannot enforce strict mode.** An MCP handler receiving "invalid
   id" needs to return a fatal `ToolError`, but receiving "LanceDB
   transient I/O" should retry. With `anyhow::Error` the only signal is
   `format!("{e}")`, which is brittle (string-matching on error messages
   is the classic anti-pattern that breaks on every prose tweak).
2. **CLI swallowed `RowNotFound` as recoverable.** Wave 1A audit
   surfaced `update_body_with_projection`'s "id not found" being wrapped
   as `StoreError(anyhow!(...))`. `is_recoverable() == true` told the
   warn-and-continue layer this was transient. It wasn't — the row never
   existed. A new `RowNotFound { id }` variant fixes the
   classification.

Typed errors let the MCP handler `match err.kind()` for strict mode and
the CLI `if err.is_recoverable() { warn() } else { abort() }` for
lenient mode — same helper, two policies, no string-matching.

### Variant taxonomy

| Variant | Recoverable? | When emitted |
|---|---|---|
| `InvalidId(String)` | no | id failed `validate_artifact_id` (path-traversal payloads, empty, illegal chars) |
| `InvalidKind { id, kind, source }` | no | frontmatter `kind` not parseable as `ArtifactKind` |
| `EmptyField { field }` | no | `Some("")` / `Some("   ")` for status or title at helper boundary |
| `FileNotFound { id, path }` | no | sync-from-file helper called for a missing markdown file |
| `ProjectionMismatch { id, kind_db, kind_file }` | no | drift between on-disk frontmatter `kind` and DB row `kind` (defined; live drift detection arrives in Phase 3d for `sync_metadata_from_file` / `sync_relation_from_file`) |
| `RowNotFound { id }` | no | input-side: caller passed an id that has no DB row (replaces misleading `StoreError` for this case) |
| `StoreError(#[from] anyhow::Error)` | **yes** | underlying `LanceStore` mutation failure — transient I/O, lock contention |

### Before / after error matrix

| Helper | Phase 3a/3b (`anyhow::Result`) | Phase 3c (`MutationResult`) |
|---|---|---|
| `create_artifact_with_projection` | `bail!("invalid id")` / `bail!("invalid kind")` / `?` on store | `InvalidId` / `InvalidKind` / `StoreError` |
| `delete_artifact_with_projection` | `bail!("invalid id")` / `?` on store | `InvalidId` / `StoreError` |
| `update_metadata_with_projection` | `bail!("status cannot be empty")` / `bail!("title cannot be empty")` / `bail!("invalid id")` | `EmptyField{status}` / `EmptyField{title}` / `InvalidId` / `StoreError` |
| `update_body_with_projection` | `bail!("invalid id")` / `bail!("not found in store")` / `?` on store | `InvalidId` / **`RowNotFound`** (was misclassified as `StoreError`) / `StoreError` |
| `update_depth_with_projection` | `bail!("invalid id")` / `?` on store | `InvalidId` / `StoreError` |
| `add_tags_with_projection` / `remove_tags_with_projection` | `bail!("invalid id")` / `?` on store | `InvalidId` / `StoreError` |
| `add_link_with_projection` / `delete_link_with_projection` | `bail!("invalid id")` x2 / `?` on store | `InvalidId` (source/target) / `StoreError` |
| `add_links_batch_with_projection` | `anyhow::Result<usize>` | `MutationResult<usize>` (per-link `InvalidId` / `StoreError`) |
| `sync_artifact_from_file` | `bail!("invalid id")` / `bail!("invalid kind")` / `?` on store | `InvalidId` / `InvalidKind` / **`FileNotFound`** (new — requires `workspace: &Path`) / `StoreError` |
| `sync_body_from_file` | `bail!("invalid id")` / `?` on store | `InvalidId` / `InvalidKind` / **`FileNotFound`** (new — requires `workspace: &Path`) / `StoreError` |
| `sync_metadata_from_file` | `bail!("invalid id")` / `bail!("status/title empty")` | `InvalidId` / `EmptyField` / `StoreError` |
| `sync_relation_from_file` | `bail!("invalid id")` x2 / `?` on store | `InvalidId` / `StoreError` |
| `delete_orphan_artifact` / `delete_orphan_relation` | `bail!("invalid id")` / `?` on store | `InvalidId` / `StoreError` |
| `delete_artifact_after_soft_delete` | `bail!("invalid id")` / `?` on store | `InvalidId` / `StoreError` |

### Downstream impact

This is **library-level** breakage only. End users of the `forgeplan`
CLI binary or the `forgeplan-mcp` server see no behavior change beyond
the audit fix in `update_body_with_projection`. The break surface is:

- **Direct consumers of `forgeplan_core::projection::*`** — `forgeplan-cli`
  (in this PR: `commands/git_sync.rs`, `commands/reindex.rs`,
  `commands/watch.rs`) and any third-party crate calling the helpers.
- **Anyhow's blanket `From<E: std::error::Error + Send + Sync + 'static>
  for anyhow::Error`** keeps `?` propagation working unchanged in
  `anyhow::Result`-returning callers — the `MutationError` is auto-wrapped.

What does **not** break:

- Callers using `?` to bubble up into an `anyhow::Result<T>` function.
- Callers using `Result<T, anyhow::Error>` explicitly — anyhow's blanket
  `From` impl handles the conversion.

What **does** break (compile-time, fast feedback):

- Callers that explicitly type the helper return as `anyhow::Result<T>`
  in a `let ...: anyhow::Result<()> = projection::foo(...)` binding.
- Callers that `match` on the returned error and pattern-match
  `anyhow::Error::downcast`-style — they should match `MutationError`
  variants directly now.

### Migration path for downstream consumers

1. **Update return types** if explicitly annotated with
   `anyhow::Result<T>` — change to `forgeplan_core::projection::MutationResult<T>`
   or rely on `?` propagation into a wider `anyhow::Result<T>`.
2. **Update `match` arms** if the caller pattern-matches error variants
   — replace string-matching on `format!("{e}")` with
   `match err { MutationError::InvalidId(id) => ..., MutationError::StoreError(_) => ..., _ => ... }`.
3. **Call `is_recoverable()`** to drive retry / warn-and-continue
   policy decisions instead of `format!("{e}").contains("transient")`.

### Architectural change: `workspace: &Path` on sync-from-file helpers

Two helpers gained a `workspace: &Path` parameter so they can construct
the full file path and emit `FileNotFound { id, path }` with the actual
on-disk location:

- `sync_artifact_from_file(workspace, store, artifact)`
- `sync_body_from_file(workspace, store, id, kind, body)`

CLI callers (`reindex.rs`, `git_sync.rs`, `watch.rs`) were updated in
this PR. External consumers calling these two helpers must thread the
workspace path through — typically already available as the `Workspace`
or `&Path` next to the store handle.

### Open work — Phase 3d (deferred)

Two items found during Phase 3c are intentionally deferred so this PR
stays scoped to the typed-error migration:

1. **Drift detection in `sync_metadata_from_file` and
   `sync_relation_from_file`** — the `ProjectionMismatch` variant is
   defined and tested in `error.rs` but not yet emitted by these two
   helpers. Adding it requires extending their signatures with `kind:
   &str` (sync_metadata) and `kind: &str` for source+target
   (sync_relation), so the helper can compare the on-disk frontmatter
   kind against the DB row's kind. Out of scope for 3c; tracked in
   PRD-073 Phase 3d.
2. **`add_links_batch` `Vec::contains` O(N²) dedup** — Wave 1B audit
   LOW-4 flagged the per-link dedup loop as quadratic. Replacement
   with `HashSet<&str>` is mechanical and orthogonal to the typed-error
   migration. Code-comment marker left in place; tracked in PRD-073
   Phase 3d.

### Status update

| Phase | Scope | Status |
|---|---|---|
| 3c | Typed `MutationError` + 16-helper migration | ✅ done (this PR): 16 helpers in `projection/mod.rs` migrated; `error.rs` extracted to its own module; `RowNotFound` variant added (Wave 1A audit fix); `FileNotFound` enabled by `workspace: &Path` on two sync-from-file helpers |
| 3d | Drift detection in `sync_metadata` / `sync_relation` + `HashSet` dedup in `add_links_batch` | ⏳ pending |
| 5 | EVID-094 supplement: clone reproducibility at CL3 + closure | ⏳ pending |

