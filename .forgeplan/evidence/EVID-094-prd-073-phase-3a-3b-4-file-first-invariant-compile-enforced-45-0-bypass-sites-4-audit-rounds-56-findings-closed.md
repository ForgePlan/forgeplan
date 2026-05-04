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
title: PRD-073 Phase 3a + 3b + 4 — file-first invariant compile-enforced (45 → 0 bypass sites, 4 audit rounds, 56 findings closed)
---

# EVID-094: PRD-073 Phase 3a + 3b + 4 (full sprint closure)

| Field | Value |
|-------|-------|
| Status | Active |
| Created | 2026-05-01 |
| Valid Until | 2027-05-01 |
| Target | PRD-073 — full closure of PROB-048 / ADR-003 invariant |
| Phases | 1 + 2 (helpers + MCP migration) + 3a (bulk CLI) + 3b (sync-mechanism extraction) + 4 (`pub(crate)` lockdown) |
| Audit rounds | 4 (general + live-test + Rust-focused + final team-lead) |
| Findings closed | 56 (7 CRITICAL + 16 HIGH + 19 MEDIUM + 14 LOW) |
| PR | [#230](https://github.com/ForgePlan/forgeplan/pull/230) |

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 2

## Measurement

Direct mutation surface measurement on feature branch
`feat/prd-073-phase-3-bulk-cli-migration` after audit remediation. Two
counts are reported: the original (single-line literal-match) ratchet, and
the corrected (whitespace-tolerant) ratchet that recognizes the
`store\n.method(` chain pattern produced by `rustfmt`.

**Method**: `tests/adr_003_invariant.rs::count_violations_in_text` enumerates
all call sites in `crates/forgeplan-cli/src/commands/` and
`crates/forgeplan-mcp/src/server.rs` (production code paths only) that
invoke any of 11 mutating `LanceStore` methods (`create_artifact`,
`update_artifact`, `update_valid_until`, `update_depth`, `update_body`,
`add_tags`, `remove_tags`, `delete_artifact`, `add_relation`,
`delete_relation`, `delete_relations_for_artifact`).

**Audit-remediation step (2026-05-01)**: the previous matcher used
`text.matches("store.method(")`, which **only counted single-line
invocations** and silently let through 7 multi-line CLI sites + 6
multi-line MCP sites that `rustfmt` had naturally wrapped. The current
matcher is a hand-rolled byte scanner that tolerates arbitrary whitespace
between `store`, `.`, the method name, and the opening paren. This was
A-AUDIT CRITICAL #6 (architect-review) / CRITICAL #4 (code-reviewer) /
N/A (security-expert) — a meta-tooling-incomplete failure where the
ratchet enforced a contract weaker than it advertised.

## Result

### Counts under the corrected (whitespace-tolerant) matcher

| Surface | Pre Phase 3a | After initial Phase 3a | After audit remediation |
|---|---|---|---|
| CLI commands/ (real count) | 34 | 21 | **17** |
| MCP server.rs (real count, prod only) | 11 | 9 | **4** |
| Combined | 45 | 30 | **21** |

Reduction: **CLI -50 %, MCP -64 %, combined -53 %**. (The original
"47 % reduction" headline was based on the single-line counter and
under-stated the actual improvement.)

### What was migrated

**Phase 3a initial commit `179bd7d`** (13 CLI + 2 MCP, single-line
visible to old matcher): capture, link (add+unlink), update
(depth/metadata/body), delete, remember (create+forget), reason (save flow),
promote, MCP `forgeplan_link`, MCP `forgeplan_discover_finding`.

**Audit-remediation commit (this evidence)** (4 CLI + 5 MCP, multi-line
that the old matcher missed): `new`, `tag` (add+remove), `generate`, MCP
`forgeplan_new`, MCP `forgeplan_update` (metadata + body), MCP
`forgeplan_capture`, MCP `forgeplan_generate`. Plus 2 new helpers
`add_tags_with_projection` / `remove_tags_with_projection`.

### Helpers (15 total, all in `forgeplan_core::projection`)

**Mutation helpers (9, Phase 3a)** — for command handlers, do
`{sync_before, mutate, render_after}`:
`create_artifact_with_projection`, `delete_artifact_with_projection`,
`update_metadata_with_projection`, `update_body_with_projection`,
`update_depth_with_projection`, `add_link_with_projection`,
`delete_link_with_projection`, `add_tags_with_projection`,
`remove_tags_with_projection`.

**Sync-from-file helpers (6, Phase 3b)** — for reindex/git_sync/watch
where the file is already authoritative, file→DB direction only:
`sync_artifact_from_file`, `sync_body_from_file`,
`sync_metadata_from_file`, `sync_relation_from_file`,
`delete_orphan_artifact`, `delete_orphan_relation`.

**Bonus** (Phase 3a follow-up): `add_links_batch_with_projection` for
bulk-import perf (deduplicates pre-sync + post-render per unique
participant — 100-link bundle goes from 600 LanceDB calls to ~2×U + N).
`delete_artifact_after_soft_delete` for the MCP soft-delete pattern.

**Phase 4 lockdown** demoted 11 mutating `LanceStore` methods to
`pub(crate)` so direct calls from `commands/*.rs` or `server.rs` now
fail compile, not just fail the regression test. `update_embedding`
and `update_r_eff_score` stay `pub` (Class A derived data per
ADR-003 Amendment 1). Test fixtures use `*_for_test` escape hatches
gated on `cfg(any(test, all(feature = "test-helpers", debug_assertions)))`
so release builds with the feature accidentally enabled still keep
the lockdown.

### Audit remediation summary (5 CRITICAL + 5 HIGH closed)

- **CRITICAL multi-line ratchet gap** — fixed: hand-rolled
  whitespace-tolerant scanner, baselines re-set honestly.
- **CRITICAL `update --depth --title` orphan-file recreation** — fixed:
  metadata helper runs FIRST so subsequent depth/body renders see the
  new title; old-slug cleanup at end via `remove_projection_at` (exact
  path) so no orphan window and no prefix collision.
- **CRITICAL `add_link` / `delete_link` lost warn-and-continue** —
  fixed: helpers now `tracing::warn!`-and-continue on target pre-sync
  and on both post-render calls; only source pre-sync and the relation
  write itself remain fatal.
- **CRITICAL C1 non-atomic file write** — fixed: `render_projection_inner`
  and `render_projection_record` now use `atomic_markdown_write` (the
  same tempfile+rename helper that `stamp_agent_identity` already used).
- **CRITICAL C2 `remove_projection` prefix collision** — fixed:
  `delete_artifact_with_projection` now resolves the file via
  `remove_projection_at(workspace, id, kind, title)` — exact path from
  the record's title, no `read_dir` ordering luck. Bare
  `remove_projection` was hardened with the trailing-hyphen
  requirement (numeric collision case).
- **HIGH `create_artifact_with_projection` slug-collision body desync** —
  fixed: helper now uses `render_projection_with_body` (force_body=true)
  so caller-supplied body wins over any pre-existing file at the slug.
- **HIGH H2 `update_body_with_projection` ordering** — fixed: file
  written FIRST, then DB; kill-mid-flow leaves the user's intended body
  on disk and reindex propagates to DB instead of overwriting it back.
- **HIGH `add_link` cosmetic target re-render** — kept as side-effect
  re-sync (drift recovery for target's other outgoing edges) and
  documented honestly in the helper docstring.
- **HIGH ADR-003 Class A claim** — Amendment 1 tightened: "any reader
  can reproduce" replaced with explicit "acceptable staleness between
  reindex cycles", caller-responsibility note added, Phase 4
  `DerivedDataWriter` trait scoped.
- **HIGH `update_metadata_with_projection(None, None)` no-op bumps** —
  fixed: helper now early-returns with debug-assert; CLI gate stays as
  defense-in-depth.

### Audit round 4 (final team-lead, post-Phase-3b/4)

Three reviewers in parallel via `TeamCreate`: architect (3 HIGH +
4 MEDIUM + 3 LOW), code-reviewer (1 MEDIUM + 4 LOW), security
(1 MEDIUM + 1 LOW + 1 INFO). All HIGH + relevant MEDIUM closed:
- A1: 8 direct unit tests for new helpers added
- A2: audit.yaml playbook downgraded from "canonical" to "REFERENCE
  EXAMPLE, requires task-tool 1.x"
- A3: CHANGELOG `[Unreleased]` entry with BREAKING / behavioral /
  Added / Fixed sections
- A4: `update_metadata_with_projection` migrated to `MutationResult<()>`
  as canary for typed-error contract
- A5: ADR-003 Amendment 1 status table updated — Phase 3b/4 ✅ done
- A6: README "Cargo features" section with test-helpers warning
- S1: `cfg(any(test, feature = "test-helpers"))` tightened to
  `cfg(any(test, all(feature = "test-helpers", debug_assertions)))` —
  release builds with the feature accidentally enabled now get ZERO
  escape-hatch methods
- A10: `import_cmd.rs` shows "N of M relations applied" + warning on
  diff — half-failed imports no longer silent
- MEDIUM-1 (code-reviewer): `sync_metadata_from_file` empty
  status/title rejection mirroring H2

### Verification (full sprint final)

- `cargo fmt -- --check` clean
- `cargo check --workspace` 0 warnings
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `cargo test --workspace --no-fail-fast` **1866 passed / 0 failed**
  (+14 audit-regression tests since pre-PRD-073 baseline of 1852)
- `cargo test -p forgeplan --test adr_003_invariant` 2 passed at
  baselines **CLI=0 / MCP=0** (multi-line scanner)
- `cargo build --release -p forgeplan`: clean
- `strings target/release/forgeplan | grep _for_test`: empty (lockdown
  preserved in release)
- **Real E2E** on temp workspaces:
  - Path traversal via `id` in import bundle: REJECTED before write
  - Path traversal via `--title "../../etc/evil"`: slugify sanitizes
  - Empty `--title ""` / `--status ""`: REJECTED at helper boundary
  - 4-concurrent `update --title TN`: ONE final file (lock + tuple
    drop-order)
  - `update --depth+--title` orphan-recreation: ONE file
  - `mem-foo` vs `mem-foo-bar` collision: sibling survives
  - Import roundtrip 3 artifacts: 3 files written + 3 DB rows
  - `kill -9` mid-write × 30 iterations: 0 zero-length files
  - CLI `delete` + `undo-last` + explicit `restore`: full recovery
  - `playbook run greenfield-kickoff`: 7/7 steps green, 6 artifacts
  - `discover start → list → show → complete`: full lifecycle
  - `claim --ttl-minutes 5` + `release`: roundtrip clean
  - Lifecycle (activate/deprecate/supersede) with evidence: gates
    enforce correctly
  - Tag/untag with frontmatter inspection: tags appear/disappear
- **Real workspace** (`.forgeplan/` 263 → 264 artifacts including
  PROB-042 cyrillic→ASCII slug rename): read clean via
  health/list/status/playbook/blocked/blindspots/search/get/score

## Interpretation

The migration **fully closes** the ADR-003 invariant violation
documented in PROB-048. All 30+ mutation paths in `commands/*.rs` and
`server.rs` production code now go through `forgeplan_core::projection::*`
helpers. The regression test ratchet (`tests/adr_003_invariant.rs`,
multi-line aware) shows CLI=0 / MCP=0 — no direct `LanceStore::*`
mutations remain in production code outside the projection helper
namespace. Phase 4 `pub(crate)` lockdown makes this a compile-time
guarantee, not just a regression-test guarantee.

The remaining 21 sites are not nominal bypasses — they are sync
mechanisms (CLI: `reindex`, `git_sync`, `import_cmd`, `watch`, `ingest`;
MCP: `forgeplan_import` bundle replay) plus the soft-delete special
case (MCP `forgeplan_delete` after `soft_delete_capture` already moved
the file to trash). For these, a direct `store.X` call **is** the
projection-rebuild flow — there is no projection to render because the
source side IS the file (or bundle) or the file already lives in
`.forgeplan/trash/` waiting on `forgeplan_undo_last`. Migrating them
requires a higher-level `import_artifact_with_projection` /
`reindex_workspace_via_projection` helper extraction (PRD-073 Phase 3b),
after which the visibility lockdown in Phase 4 (demoting the 10 mutating
`LanceStore` methods to `pub(crate)`) becomes mechanical.

PROB-048 acceptance criteria status:

- [x] **Helpers exist** — 9 `*_with_projection` helpers in `core::projection`
- [partial] **Baselines lowered** — CLI 34→17, MCP 11→4 under honest
  whitespace-tolerant counting (target 0/0 reachable after Phase 3b)
- [ ] **`pub(crate)` lockdown** — deferred to Phase 4 (after Phase 3b)
- [partial] **End-to-end clone reproducibility** — not yet measured as a
  full `git clone → reindex → diff` workflow on a sample repo. EVID-094
  measures the ratchet count + helper composition + per-CRITICAL
  reproductions on temp workspaces, not byte-stable git clones.
  Full clone reproducibility EVID waits on Phase 3b/4.

## Congruence Level Justification

**CL2 (related context, penalty 0.1)**: the measurement is taken on
the same code base, same invariant, and same regression-guard test that
PRD-073 / PROB-048 / ADR-003 describe. However, the measurement is a
**count derived from a regex-style scanner**, not a direct runtime
assertion of file-vs-LanceDB ordering. A future scanner-evasion (e.g.
`store_alias.create_artifact(...)` via `let store_alias = &store`) would
slip past the count even under the corrected matcher. The architect-
review audit (2026-05-01) flagged this as overclaim of CL3 in the
initial EVID — downgraded here to CL2 to be honest about the proxy.

A future EVID at CL3 would require either (a) a runtime test that
crashes if any non-helper call mutates LanceStore tables outside the
projection layer (e.g. via a scoped `store-mutation-trace` test feature),
or (b) the Phase 4 visibility lockdown that makes scanner evasion
impossible at compile time.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-073 | informs |
| PROB-048 | informs |
| ADR-003 | informs |

