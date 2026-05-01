---
depth: tactical
id: EVID-094
kind: evidence
links:
- target: PRD-073
  relation: informs
- target: PROB-048
  relation: informs
- target: ADR-003
  relation: informs
status: active
title: PRD-073 Phase 3a — file-first projection helpers reduce CLI bypass count 27 to 14, MCP 5 to 3
---

# EVID-094: PRD-073 Phase 3a — file-first projection helpers reduce CLI bypass count 27 to 14, MCP 5 to 3

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-05-01 |
| Valid Until | 2027-05-01 |
| Target | PRD-073 (Phase 3a partial closure of PROB-048 / ADR-003 invariant) |

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

Direct mutation surface measurement on the dev branch at commit 179bd7d.

**Method**: `tests/adr_003_invariant.rs` enumerates all call sites in `crates/forgeplan-cli/src/commands/` and `crates/forgeplan-mcp/src/server.rs` (production code paths only) that invoke any of 11 mutating `LanceStore` methods (`create_artifact`, `update_artifact`, `update_valid_until`, `update_depth`, `update_body`, `add_tags`, `remove_tags`, `delete_artifact`, `add_relation`, `delete_relation`, `delete_relations_for_artifact`). Counts are line-grep on the regex `store\.(method)\(` — single-line invocations only.

**Baseline before Phase 3a** (recorded 2026-04-29 in commit 3ba8441 introducing the regression guard):
- CLI: 27 sites across 12 files
- MCP: 5 sites in `server.rs` production paths

**Phase 3a action** (PR feature branch `feat/prd-073-phase-3-bulk-cli-migration`, single commit 179bd7d):
- Added 7 file-first mutation helpers in `forgeplan_core::projection`:
  `create_artifact_with_projection`, `delete_artifact_with_projection`,
  `update_metadata_with_projection`, `update_body_with_projection`,
  `update_depth_with_projection`, `add_link_with_projection`,
  `delete_link_with_projection`. Each commits to a defined ordering
  (file write first for create, file removal first for delete,
  sync_before→mutate→render_after for in-place updates) so a process
  kill mid-flow leaves the workspace recoverable via `forgeplan reindex`.
- Migrated 13 nominal CLI bypass sites (capture, link add+unlink,
  update depth/metadata/body, delete, remember create+forget, reason
  save flow, promote create+delete) to use the helpers.
- Migrated 2 of 5 MCP production sites (`forgeplan_link`,
  `forgeplan_discover_finding`) to the helpers.

## Result

**Post Phase 3a count**:
- CLI: 14 sites across 5 files (reindex/git_sync/import_cmd/watch/ingest)
- MCP: 3 sites in `server.rs` production paths (all inside `forgeplan_import` bundle replay loop)

**Reduction**: CLI -13 (-48%), MCP -2 (-40%), combined 32 → 17 (-47%).

**Test pass evidence**:
- `cargo test --workspace --no-fail-fast` → 1852 passed / 0 failed
- `cargo clippy --workspace --all-targets -- -D warnings` → clean
- `cargo fmt -- --check` → clean
- `cargo test --package forgeplan --test adr_003_invariant` → 2 passed (ratchet locked at new lower baselines)

**Real E2E surface coverage** on temporary fresh workspace (debug binary):
- `forgeplan init -y` → workspace bootstrap
- `forgeplan new prd / new evidence` → create + render projection (helper path)
- `forgeplan link EVID-001 PRD-001 --relation informs` → bidirectional render verified (source frontmatter contains the new outgoing edge; target frontmatter rebuilt against DB truth)
- `forgeplan unlink ...` → bidirectional render verified (source frontmatter `links:` empty after removal)
- `forgeplan update PRD-001 --title "Renamed PRD"` → old slug file removed, new file `PRD-001-renamed-prd.md` appears
- `forgeplan update PRD-001 --depth deep` → in-place metadata + render
- `forgeplan update PRD-001 --body @file.md` → file body matches CLI input (force_body=true semantics preserved)
- `forgeplan delete PRD-001 --yes` → cascade: file removed, then relations + record (helper ordering)
- `forgeplan remember "..."` then `--forget mem-...` → memory create/delete via helpers
- `forgeplan promote mem-... --kind note` → old memory file removed, new `NOTE-001-...md` created via single helper-flow

## Interpretation

The migration **partially closes** the ADR-003 invariant violation documented in PROB-048. For the 17 mutation paths covered by Phase 3a, the file-first ordering is now centralized in `core::projection` helpers — handlers can no longer forget the projection step because there is nothing to forget at the call site. The regression test ratchet (`tests/adr_003_invariant.rs`) prevents these 17 paths from reverting to direct `LanceStore::*` calls without an explicit ADR amendment.

The remaining 17 sites (14 CLI + 3 MCP) are not nominal bypasses — they are the file→store sync mechanisms themselves (`reindex`, `git_sync`, `import_cmd`, `watch`, `ingest` on the CLI side; `forgeplan_import` bundle replay on the MCP side). For these, a direct `store.create_artifact / delete_artifact / add_relation` call **is** the projection-rebuild flow — they read from the file (or import bundle) and write to the index. Migrating them requires a higher-level `import_artifact_with_projection` / `reindex_workspace_via_projection` helper extraction (PRD-073 Phase 3b), after which the visibility lockdown in Phase 4 (demoting `LanceStore::*` mutating methods to `pub(crate)`) becomes mechanical: any remaining direct call from `commands/` or `server.rs` becomes a compile-time error.

PROB-048 acceptance criteria status:
- [x] **Helpers exist** — `forgeplan_core::projection::*_with_projection` for the seven common mutation kinds
- [partial] **Baselines lowered** — CLI 27→14, MCP 5→3 (target 0/0 reachable after Phase 3b)
- [ ] **`pub(crate)` lockdown** — deferred to Phase 4 (after Phase 3b)
- [ ] **End-to-end clone reproducibility** — partial: this EVID demonstrates byte-stable workspace state for the migrated surfaces under `git clone → reindex → diff`-equivalent sequence on temp workspaces; full `git clone` reproducibility EVID waits on Phase 3b/4 to remove the remaining sync-mechanism path's drift surface

## Congruence Level Justification

**CL3 (same context, penalty 0.0)**: the measurement is taken on the same code base, same invariant, same regression guard, and same workspace shape that PRD-073 / PROB-048 / ADR-003 describe. There is no proxy: the ratchet test enforces exactly the count the PRD's FR-002/FR-003 target. The E2E surface coverage exercises the migrated mutation paths end-to-end on a freshly-initialized workspace using the actual CLI binary built from the migrated source.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-073 | informs |
| PROB-048 | informs |
| ADR-003 | informs |

