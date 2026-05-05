---
depth: standard
id: PROB-049
kind: problem
last_modified_at: 2026-05-02T11:31:38.722305+00:00
last_modified_by: claude-code/2.1.126
links:
- target: PRD-073
  relation: based_on
status: draft
title: Phase 3d follow-ups — typed errors deferrals from PRD-073 Phase 3c R1/R2 audits
---

# PROB-049: Phase 3d follow-ups — typed errors deferrals from PRD-073 Phase 3c R1/R2 audits

## Signal

PRD-073 Phase 3c shipped 16 helper migrations to `MutationResult<T>` plus 2
audit rounds (4+3 specialized agents, opus). 8 R1 + 6 R2 findings were closed
in-flight; the remaining items are surface-level deferrals that don't block
the Phase 3c PR but should be tracked as a coherent Phase 3d sprint instead
of orphan TODO comments scattered across `crates/forgeplan-core/src/projection/`.

Concretely, the following deferred findings carry `TODO(PROB-049)` markers
in code:

1. **H-1 (rust+architect+security)** — `MutationError::StoreError(#[from] anyhow::Error)`
   is over-broad: schema corruption, missing-table, malformed predicate, AND
   transient I/O all collapse into one variant marked `is_recoverable() ==
   true`. MCP retry loops would hammer LanceDB on permanent failures. Split
   into `StoreTransient` (recoverable) vs `StoreFatal` (not).

2. **H-2 (rust+architect+security)** — Asymmetric missing-row policy:
   `delete_artifact_with_projection` returns `Ok(())` for missing id (idempotent),
   `update_body_with_projection` returns `RowNotFound` (input error). Documented
   as intentional in code, but a unified contract would simplify caller logic.

3. **H-4 (code-review)** — Zero `# Errors` rustdoc sections across 16
   migrated helpers. Standard rustdoc convention; caller has to read the
   body to learn which `MutationError` variants apply. Mechanical doc add.

4. **H-6 (architect)** — Three styles of helper signatures (path-aware,
   path-blind, hybrid). `MutationContext { workspace: &Path, store: &LanceStore }`
   would unify; Phase 3d signature refactor.

5. **M-2 (rust)** — `match Some/None` blocks in `delete_artifact_with_projection`
   and `update_body_with_projection` should be `let-else` (idiomatic since 1.65).
   Stable; deferred.

6. **M2-1 (rust R2)** — 4 `InvalidKind` `.map_err` closures repeat the same
   shape. Centralize via `fn invalid_kind(id, kind, e) -> MutationError` helper.

7. **M2-2 (rust R2)** — 2 `match tokio::fs::metadata` arms duplicate verbatim
   in `sync_artifact_from_file` / `sync_body_from_file`. Extract `fn
   ensure_file_exists(workspace, path, id) -> MutationResult<()>`.

8. **M-7 (rust)** — Redundant `let _ = remove_projection_at(...).await?;`.

9. **M-9 (rust)** — Several typed-error tests assert only the variant, not
   "AND no side effect happened" (model: `add_links_batch_returns_invalid_id_before_any_write`).

10. **M-11 (architect)** — `crates/forgeplan-core/src/projection/mod.rs` is
    2,800+ lines with 100+ tests in one `mod tests`. Split tests into
    `projection/tests/` directory per helper cluster.

11. **M-12 (architect)** — `LanceStore` discards `workspace` after deriving
    `lance/`. Storing `workspace: PathBuf` on `LanceStore` and exposing
    `pub fn workspace(&self) -> &Path` would let helpers drop the
    `workspace: &Path` parameter.

12. **M-R2-3 (security R2)** — `MutationError::FileNotFound { path: PathBuf }`
    field stays publicly constructible. Future contributor could pass an
    absolute path. Constructor or `debug_assert!(!path.is_absolute())` would
    make the contract type-enforced.

13. **L-R2-3 (security R2)** — `InvalidKind { reason: String }` is populated
    via `e.to_string()` where `e: ForgeplanError`. Today safe (only
    `ForgeplanError::InvalidKind(s)` reaches here), but if `ArtifactKind::FromStr::Err`
    ever returns `Io`/`Yaml`, raw OS messages could leak. Tighten with
    explicit variant match.

14. **L-1 / TODO ticket references** — Code comments mark deferrals as
    "Phase 3d" without a tracking artifact. This very PROB closes that
    loop — every TODO marker now has a real ID to grep for.

## Constraints

- MUST NOT break the public `forgeplan-core` API again — Phase 3d is a
  series of refactors that are individually safe; aggregate them in one
  release notes block to give downstream library consumers a clean upgrade
  story.
- MUST keep `MutationResult<T>` contract internal to `projection::*`
  (ADR-003 Amendment 2 — typed errors stay inside the projection layer,
  callers consume via `?`/anyhow blanket From).
- MUST run audit (4+ agents) on the Phase 3d PR — same rigor as Phase 3c.

## Optimization Targets (1-3 max)

- **MCP strict-mode correctness**: split `StoreError` so retry loops
  don't amplify permanent failures (H-1 is the highest-value item).
- **Documentation density**: add `# Errors` rustdoc to 16 helpers (H-4).
- **Code organization**: file size + signature unification (M-11/H-6).

## Observation Indicators (Anti-Goodhart)

- Test count: must stay ≥ Phase 3c baseline (1894 lib + ~370 integration).
  Don't game the metric by deleting tests during the file split.
- `cargo clippy --workspace --all-targets -- -D warnings` clean: 0 warnings
  before AND after each Phase 3d sub-PR.
- `forgeplan health`: blind_spots / orphans / stale stays at 0.

## Acceptance Criteria

- [ ] H-1 `StoreError` split shipped + tests for each new variant
- [ ] H-2 missing-row policy documented OR unified (lead choice)
- [ ] H-4 `# Errors` rustdoc on all 16 migrated helpers
- [ ] H-6 unified `MutationContext` (or explicit decision against)
- [ ] All M-class items closed via small PRs OR explicitly accepted
- [ ] All `TODO(PROB-049)` markers in code resolved or relabeled
- [ ] One audit round (4 agents) on the aggregate Phase 3d delta

## Blast Radius

- `forgeplan-core` library API (downstream consumers see new variants —
  additive, not breaking, because `match` arms today must already use
  `_ =>` per the variant taxonomy contract).
- `forgeplan-cli` and `forgeplan-mcp` unaffected (consume via `?` /
  anyhow blanket From; behavior identical at the CLI/MCP boundary).

## Reversibility

medium — Phase 3d is a series of refactors landed as small independent PRs.
Each PR is reversible via revert. The `StoreError` split (H-1) is the only
item with a downstream-visible `match` API change; everything else is
internal restructuring or documentation.

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-073 | based_on (parent — closes Phase 3c, this is the open-work follow-up) |
| ADR-003 | informs (Amendment 2 documents the variant taxonomy + Phase 3d direction) |
| EVID-094 | informs (PR #230 baseline measurement, pre-Phase 3c) |


