---
depth: standard
id: EVID-095
kind: evidence
last_modified_at: 2026-05-02T11:40:03.411672+00:00
last_modified_by: claude-code/2.1.126
links:
- target: PRD-073
  relation: informs
- target: ADR-003
  relation: informs
status: draft
title: Phase 3c sprint closure — typed MutationResult migration with 2-round audit (Wave 1+2 + R1+R2 fixes)
---

# EVID-095: Phase 3c sprint closure — typed MutationResult migration with 2-round audit (Wave 1+2 + R1+R2 fixes)

| Field | Value |
|-------|-------|
| Status | Active |
| Created | 2026-05-02 |
| Valid Until | 2026-08-02 (90 days) |
| Target | PRD-073 (Phase 3c sub-deliverable), ADR-003 (Amendment 2) |

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

Phase 3c sprint executed 2026-05-02 on branch `feat/prd-073-phase-3c-typed-errors`
(based on dev @ ab28bf2). Multi-agent dispatch via TeamCreate Mode A
(file-partitioned, opus model) + 2 audit rounds (4+3 specialized agents,
opus, adversarial directive).

**Sprint structure:**

| Phase | Agents | Output |
|-------|--------|--------|
| Pre-Wave 0 | Lead | Extract `MutationError`/`MutationResult` to `projection/error.rs`; add `FileNotFound` + `ProjectionMismatch` variants; 7 unit tests |
| Wave 1A | rust-w1a (rust-pro/opus) | 5 helpers migrated + 8 tests + LOW-5 slug fallback |
| Wave 1B | rust-w1b (rust-pro/opus) | 5 helpers migrated + 5 FileNotFound tests + 3 CLI call-site updates (workspace param) |
| Wave 1C | rust-w1c (rust-pro/opus) | 6 helpers migrated + 7 InvalidId/EmptyField tests |
| Lead post-W1 | Lead | RowNotFound variant + update_body fix + test rename |
| Wave 2 | adr-architect (opus) | ADR-003 Amendment 2 (+141 lines) + CHANGELOG (+19 lines) + PRD-073 progress |
| Audit R1 | rust-pro + security-expert + architect-reviewer + code-reviewer (4×opus) | 1C + 10H + 13M + 10L = 34 findings |
| R1 fix-batch | Lead | 8 findings closed (C-1, H-3, H-5, H-7, H-8, M-1, M-3, M-13) + 2 TODO markers |
| Audit R2 | rust-pro + code-reviewer + security-expert (3×opus, uplift only) | 0C + 0H + 6M + 6L; sign-offs confirmed all R1 fixes closed |
| R2 fix-batch | Lead | 6 findings closed (M-R2-1×2 [code+sec], M-R2-2 [code+sec], M-R2-3 [sec], L-R2-1) + PROB-049 created |

**Surface measured:**

- 16 helpers in `crates/forgeplan-core/src/projection/mod.rs` migrated to `MutationResult<T>`
- `projection/error.rs` extracted as separate module (146 lines, 7 variants, 8 unit tests)
- 3 CLI call-sites updated for `workspace: &Path` threading (`git_sync.rs`, `reindex.rs`, `watch.rs`)
- 4 commits on the sprint branch:
  - `d81d0ac` — Pre-Wave 0 extract
  - `8f2c0f3` — Phase 3c main (16 helpers + Amendment 2)
  - `5de4f05` — R1 fix-batch
  - `a22de47` — R2 fix-batch + PROB-049

## Result

**Quantitative metrics:**

| Metric | Pre-Phase-3c | Phase 3c shipped | Delta |
|--------|--------------|------------------|-------|
| Library tests | 1866 | 1452 (lib only) | — (counts vary by feature flag combos; the load-bearing fact is **0 failures** at every gate) |
| `projection::*` lib tests | 35 (pre-PR230 + canary baseline) | 68 | +33 |
| Helpers using `MutationResult<T>` | 1 (canary) | 17 | +16 |
| `MutationError` variants | 4 (initial) | 7 | +3 (FileNotFound, ProjectionMismatch, RowNotFound) |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean | clean | — |
| `cargo fmt --check` | 0 diffs | 0 diffs | — |
| Audit findings closed (R1+R2) | — | 14 (1C+5H+8M) | — |
| Audit findings deferred (with TODO + PROB-049 tracking) | — | ~11 | — |

**Qualitative outcomes:**

- ADR-003 Amendment 2 documents the variant taxonomy + Phase 3d open work
- CHANGELOG `[Unreleased]` entry under `### Changed` (BREAKING for downstream
  library consumers) with concrete migration example
- CHANGELOG behavior-change note for Cyrillic title persistence (R2 M-R2-3)
- PROB-049 created to track 14 deferred items; `TODO(PROB-049)` markers in
  code make `grep` surface them with a real tracking ID
- Multi-agent dispatch pattern (Mode A, opus, file partitioning) proven again
  for shared-file work (3 sub-agents on `mod.rs`, zero merge conflicts)

**Real-CLI E2E validation:**

Tested on fresh `/tmp/phase3c-e2e` workspace via release binary:
- `forgeplan new prd "Phase 3c E2E smoke test"` → create_artifact_with_projection ✓
- `forgeplan update PRD-001 --title "Updated Title 3c"` → update_metadata_with_projection (canary) ✓
- `forgeplan link PRD-001 PRD-002 --relation informs` → add_link_with_projection ✓
- `forgeplan tag PRD-001 smoke e2e` → add_tags_with_projection ✓
- `forgeplan delete PRD-002 --yes` → delete_artifact_with_projection (soft-delete receipt) ✓
- `forgeplan reindex` → sync_artifact_from_file + sync_relation_from_file
  (orphan relation cleanup) ✓
- `forgeplan update '../../etc/passwd' --title "evil"` → InvalidId at CLI boundary
  with hint contract `Fix: forgeplan list` ✓

## Interpretation

The Phase 3c migration delivers the typed-error contract that PRD-073
Phase 3a/3b set up — every mutation helper in `projection::*` now returns
`MutationResult<T>` with semantically distinct, unrecoverable-by-default
variants. MCP can build a strict-mode client on top of `is_recoverable()`
without string-matching error messages. CLI keeps its lenient
warn-and-continue posture via the same `?` ergonomics.

Two CRITICAL classes of latent bugs were caught and fixed during the audit:

1. **Slug-fallback drift (R1 C-1)** — file/DB path divergence for non-ASCII
   titles would have produced spurious `FileNotFound` for every reindex on
   Cyrillic/CJK workspaces. Fixed via centralized `projection_slug()` helper.
2. **Absolute-path leak in MCP error JSON (R1 H-8)** — the `FileNotFound`
   variant embedded the user's home directory in error messages routed
   through MCP / Claude Desktop transcripts. Fixed by stripping workspace
   prefix at construction; R2 hardened the strip to never silently fall
   back to absolute path under symlink/canonicalization mismatch.

Both were code-correct in isolation (Wave 1 sub-agents implemented to spec)
but emerged as system-level bugs because the **specification itself had a
gap** — the slug-fallback was only specified for `create_artifact`, and
the path-stripping was specified without considering symlink edge cases.
Multi-agent execution surfaces these gaps faster because each agent
implements within its narrow scope and the audit lens catches the
cross-helper inconsistencies the lead would otherwise miss.

The Phase 3d backlog (PROB-049, 14 items) is a coherent follow-up sprint,
not a list of bugs — items are mostly internal refactors (helper signature
unification, file split, doc density), with one downstream-visible API
addition (`StoreError` split into `StoreTransient`/`StoreFatal`). All
tracked with `TODO(PROB-049)` markers in code so `grep` surfaces them.

## Congruence Level Justification

CL3 (same context, penalty 0.0) — this evidence is the direct measurement
of the sprint that PRD-073 Phase 3c describes. The branch
`feat/prd-073-phase-3c-typed-errors` IS the Phase 3c implementation; the
test counts, audit findings, and merged commits are the artifacts of
that implementation. There is no proxy / external study / related-context
gap.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-073 | supports |
| ADR-003 | supports |
| EVID-094 | informs (PR #230 baseline measurement, pre-Phase-3c) |
| PROB-049 | informs (Phase 3d follow-up tracker, created from these audits) |


