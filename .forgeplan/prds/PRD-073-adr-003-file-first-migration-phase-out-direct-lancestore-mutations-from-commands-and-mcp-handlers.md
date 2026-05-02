---
depth: standard
id: PRD-073
kind: prd
links:
- target: PROB-048
  relation: based_on
- target: ADR-003
  relation: based_on
- target: PROB-048
  relation: based_on
status: draft
title: ADR-003 file-first migration — phase out direct LanceStore mutations from commands and MCP handlers
---

# PRD-073: ADR-003 file-first migration

## Problem

PROB-048 documents 32+ direct `LanceStore` mutating call sites in `crates/forgeplan-cli/src/commands/` (27) and `crates/forgeplan-mcp/src/server.rs` (5 production). Each is a potential ADR-003 invariant violation: writes go to LanceDB without first updating the markdown file. Three concrete drifts surfaced in the 2026-04-28 sprint (deprecate, link bidirectional, status).

## Goals

1. **Zero direct mutations**. All 32 call sites migrated to a single helper API that writes the markdown file first, then syncs LanceDB.
2. **Compile-time enforcement**. After migration, demote `LanceStore::{create_artifact, update_*, delete_*, add_relation, delete_relation}` to `pub(crate)`. External consumers (commands, MCP handlers) cannot bypass.
3. **Regression-proof**. The existing `tests/adr_003_invariant.rs` baseline reaches `CLI_BASELINE = 0` / `MCP_BASELINE = 0` and stays there. Visibility change makes any future bypass a compile error.
4. **Reproducibility evidence**. EVID with `git clone → forgeplan reindex → assert workspace state identical to source` measurement.

## Non-Goals

- NOT a rewrite of `LanceStore` schema or query API — only mutation surface
- NOT touching read-only methods (`get_*`, `list_*`, `search_*`) — they don't violate the invariant
- NOT tackling `update_embedding` or `update_r_eff_score` — these are derived/cached values where LanceDB IS the authoritative store (covered in separate ADR clarification if needed)
- NOT FPF or vector-search code paths — `insert_fpf_chunks` etc. operate on a separate LanceDB table without markdown projection

## Target Users

- **Forgeplan contributors** writing new commands or MCP handlers — get a single obvious helper, no decision paralysis
- **Brownfield adopters** running scan-import on existing repos — clone reproducibility ensures their workspace state survives `git pull` without surprises
- **AI agents** (Claude Code et al) using forgeplan tools — deterministic file state across sessions

## Functional Requirements

- **FR-001**: New helper `forgeplan_core::projection::write_artifact_mutation(workspace, id, mutation_kind, fields)` that:
  - Reads current file
  - Applies mutation to in-memory frontmatter / body
  - Writes file atomically
  - Calls `reindex_one(path)` to sync LanceDB
- **FR-002**: Migrate CLI commands (27 call sites). Lower `CLI_BASELINE` in regression test to `0`.
- **FR-003**: Migrate MCP handlers (5 call sites). Lower `MCP_BASELINE` to `0`.
- **FR-004**: Visibility change — `LanceStore::create_artifact / update_* / delete_* / add_relation / delete_relation` → `pub(crate)`. External crates cannot call.
- **FR-005**: Bidirectional link rendering — `forgeplan link` re-renders projections for source AND target so both files reflect the edge.
- **FR-006**: EVID-XXX measurement — clean `git clone` of a repo with active artifacts, run `forgeplan reindex`, compare workspace state byte-for-byte to source.

## Success Criteria

1. `tests/adr_003_invariant.rs` passes with both baselines = 0
2. `cargo build -p forgeplan-cli -p forgeplan-mcp` fails to compile if one re-introduces a direct mutation (because methods are `pub(crate)`)
3. `cargo test --workspace` 0 failures (existing 1400+ lib tests + new helper tests)
4. EVID demonstrates clone reproducibility with structured fields CL3
5. CHANGELOG entry для release что shipnет migration

## Phases

### Phase 1 — Helper API design + first migration (1 PR, ~2-3h)
- Design `write_artifact_mutation` signature with all mutation kinds (create, update_status, update_title, update_body, update_depth, update_valid_until, update_tags, add_link, delete_link, delete)
- Implement in `core::projection`
- Migrate 1-2 representative commands as canary (e.g., `tag.rs`)
- Lower `CLI_BASELINE` accordingly

### Phase 2 — MCP handlers + bidirectional links (1 PR, ~2h)
- Migrate 5 MCP production sites
- Implement bidirectional link rendering in helper
- Lower `MCP_BASELINE` to 0

### Phase 3 — Bulk CLI migration (1 PR, ~3-4h)
- Migrate remaining ~25 CLI sites
- Each migration is a focused diff
- `CLI_BASELINE = 0`

### Phase 4 — Visibility lockdown (1 PR, ~30min)
- Demote LanceStore methods to `pub(crate)`
- Compile errors confirm completeness
- Remove regression guard test (compile-time enforcement supersedes it) OR keep as belt-and-suspenders

### Phase 5 — Evidence + closure (1 PR, ~1h)
- Create EVID-XXX with reproducibility measurement
- Activate PROB-048 closure
- Update CLAUDE.md to remove "in-progress migration" caveat

Total: ~9-12h focused work across 5 PRs / 1 sprint.

## Related Artifacts

- PROB-048: documents the invariant violation (this PRD addresses)
- ADR-003: invariant definition (this PRD enforces)
- EVID-XXX: end-to-end reproducibility measurement (TBD)

## Progress

- [x] Phase 1 — Helper API design + canary (`update_metadata_with_projection`)
- [x] Phase 2 — MCP handlers + bidirectional links
- [x] Phase 3a — Bulk CLI migration + adversarial audit remediation (EVID-094)
- [x] Phase 3b — Sync-mechanism extraction (`sync_*_from_file` + `delete_orphan_*`)
- [x] Phase 4 — `pub(crate)` visibility lockdown (compile-time enforcement)
- [x] **Phase 3c — Typed `MutationError` + 16-helper migration** (2026-05-02)
  - 16 `projection::*` helpers migrated from `anyhow::Result<T>` to `MutationResult<T>`
  - `error.rs` extracted as a stable, low-conflict module (Wave 1A/1B/1C parallel work)
  - 7 variants finalised: `InvalidId`, `InvalidKind`, `EmptyField`, `FileNotFound`, `ProjectionMismatch`, `RowNotFound`, `StoreError`
  - Wave 1A audit fix: `update_body_with_projection` now returns `RowNotFound` instead of misleading `StoreError`
  - `sync_artifact_from_file` / `sync_body_from_file` take `workspace: &Path` to construct `FileNotFound { id, path }`
  - ADR-003 Amendment 2 records before/after taxonomy + downstream migration path
- [ ] Phase 3d — drift detection in `sync_metadata_from_file` / `sync_relation_from_file` + `HashSet` dedup in `add_links_batch_with_projection` (Wave 1B audit LOW-4)
- [ ] Phase 5 — EVID-094 supplement: clone reproducibility at CL3 + closure

