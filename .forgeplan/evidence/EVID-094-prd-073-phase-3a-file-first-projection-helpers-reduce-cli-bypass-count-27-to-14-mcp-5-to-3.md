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
title: PRD-073 Phase 3a (post-audit) — file-first projection helpers + multi-line ratchet fix → 24 nominal bypass sites migrated, baselines CLI 17 / MCP 4 (sync-mechanism only)
valid_until: 2027-05-01
---

# EVID-094: PRD-073 Phase 3a (post-audit)

| Field | Value |
|-------|-------|
| Status | Active |
| Created | 2026-05-01 |
| Valid Until | 2027-05-01 |
| Target | PRD-073 (Phase 3a partial closure of PROB-048 / ADR-003 invariant), incorporating remediation of architect-review + code-reviewer + security-expert findings |

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

### Helpers (9 total, all in `forgeplan_core::projection`)

`create_artifact_with_projection`, `delete_artifact_with_projection`,
`update_metadata_with_projection`, `update_body_with_projection`,
`update_depth_with_projection`, `add_link_with_projection`,
`delete_link_with_projection`, `add_tags_with_projection`,
`remove_tags_with_projection`.

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

### Verification (post-remediation)

- `cargo fmt -- --check` clean
- `cargo check --workspace` 0 warnings
- `cargo clippy --workspace --all-targets -- -D warnings` clean
- `cargo test --workspace --no-fail-fast` **1851 passed / 0 failed**
- `cargo test -p forgeplan --test adr_003_invariant` 2 passed at honest
  baselines (CLI 17 / MCP 4)
- **Real E2E reproduction of each fixed CRITICAL** on temp workspace:
  - CRITICAL #2: `forgeplan update PRD-001 --depth deep --title "Renamed PRD"` →
    only `PRD-001-renamed-prd.md` exists (old-slug file gone, no orphan)
  - C2: `forgeplan remember "foo" + remember "foo bar" + remember --forget mem-foo` →
    `mem-foo-bar-...md` survives (sibling not clobbered)
  - H2: `forgeplan update PRD-001 --body @file` → file body matches
    CLI input, frontmatter preserved
- Full surface E2E: capture / link+unlink / tag+untag (new helpers) /
  delete cascade / remember+forget / promote — all clean

## Interpretation

The migration **partially closes** the ADR-003 invariant violation
documented in PROB-048. For the 24 mutation paths covered by Phase 3a
post-audit, the file-first ordering is now centralized in 9
`core::projection` helpers — handlers can no longer forget the
projection step because there is nothing to forget at the call site. The
regression test ratchet (`tests/adr_003_invariant.rs`), now multi-line
aware, prevents these 24 paths from reverting to direct `LanceStore::*`
calls without an explicit ADR amendment.

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
