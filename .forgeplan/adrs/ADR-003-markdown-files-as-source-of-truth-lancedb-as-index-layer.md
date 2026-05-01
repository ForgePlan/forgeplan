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

### Status of the migration (2026-05-01, post-audit)

| Phase | Scope | Status |
|---|---|---|
| 1 | Helper API design + canary migration | ✅ done (commit on dev) |
| 2 | MCP lifecycle handlers + bidirectional links | ✅ done (PR #227) |
| 3a | Bulk migration of all visible bypasses + audit remediation | ✅ done (EVID-094): 17 CLI + 7 MCP nominal bypass sites migrated through 9 helpers; multi-line ratchet fixed; atomic writes, prefix-collision, --depth+--title ordering, warn-and-continue semantics, file-first body ordering all corrected per A-AUDIT findings |
| 3b | Sync-mechanism extraction (reindex / git_sync / import_cmd / watch / ingest / forgeplan_import) → higher-level helpers + migration | ⏳ planned — current ratchet baselines: CLI 17 / MCP 4 |
| 4 | `pub(crate)` visibility lockdown for `LanceStore::create_artifact / update_artifact / update_body / update_depth / add_tags / remove_tags / delete_artifact / add_relation / delete_relation / delete_relations_for_artifact` | ⏳ blocked on 3b. `update_embedding` and `update_r_eff_score` remain `pub` per Class A above, fronted by `DerivedDataWriter` trait |
| 5 | Closure EVID with `git clone → reindex → diff` reproducibility (CL3 evidence pack) | ⏳ partial — EVID-094 covers Phase 3a surfaces at CL2 |

This amendment does not change the original decision (markdown is the
source of truth, LanceDB is the derived index). It clarifies which method
families are part of the invariant and which are deliberate carve-outs so
contributors can read the regression guard's `FORBIDDEN_PATTERNS` and
understand both what it covers and what it intentionally does not.

