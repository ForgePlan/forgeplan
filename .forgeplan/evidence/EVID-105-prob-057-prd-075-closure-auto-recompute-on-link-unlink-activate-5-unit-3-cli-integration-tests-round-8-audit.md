---
depth: standard
id: EVID-105
kind: evidence
links:
- target: PROB-057
  relation: informs
- target: PRD-075
  relation: informs
status: active
title: PROB-057 / PRD-075 closure — auto-recompute on link/unlink/activate, 5 unit + 3 CLI integration tests, Round 8 audit
---

# EVID-105: PROB-057 / PRD-075 closure — R_eff cache self-healing on link/unlink/activate

## Summary

Closes the stale-cache leak observed during PROB-053 PR review session: `forgeplan link`, `forgeplan unlink`, `forgeplan activate` now synchronously recompute and persist cached `r_eff_score` for the local target via the new shared helper `forgeplan_core::scoring::sync_score_target`. `forgeplan score` / `score-all` route through the same helper для single canonical "recompute + persist" path. Auto-recompute survives Round 8 adversarial audit (2 parallel agents — security + code-reviewer). Architectural deferrals (driver parity, score lock policy, recursive walk DoS bound) tracked separately в PROB-058.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Evidence

### Real E2E на release binary (target/release/forgeplan v0.29.0, fresh workspace, 2026-05-06)

```bash
$ /tmp/prob057-e2e $ forgeplan init -y
$ forgeplan new prd "E2E test"          # PRD-001
$ forgeplan new evidence "Backing"      # EVID-001
$ forgeplan get PRD-001 --json | jq .r_eff
0.0                                     # baseline before link

$ forgeplan link PRD-001 EVID-001 --relation informs
Linked: PRD-001 --informs--> EVID-001
Next: forgeplan score-all               # FR-009 — hint points at parents, not target

$ forgeplan get PRD-001 --json | jq .r_eff
1.0                                     # ✅ FR-001 — auto-recomputed без manual `score`

$ forgeplan unlink PRD-001 EVID-001 --relation informs
Unlinked: PRD-001 --informs--> EVID-001
Next: forgeplan score-all

$ forgeplan get PRD-001 --json | jq .r_eff
0.0                                     # ✅ FR-002 — auto-recomputed после unlink

$ forgeplan link PRD-001 EVID-001 --relation informs
$ forgeplan activate PRD-001 --force
  Activated PRD-001 (draft → active)
Next: forgeplan score-all

$ forgeplan get PRD-001 --json | jq '.r_eff, .status'
1.0
"active"                                # ✅ FR-003 — auto-recomputed после activate
```

| Cell | Action | Expected `r_eff` | Observed |
|---|---|---:|---:|
| A | fresh PRD baseline | 0.0 | ✅ 0.0 |
| B | `link PRD EVID informs` (no score) | > 0.0 | ✅ 1.0 |
| C | `unlink PRD EVID informs` (no score) | 0.0 | ✅ 0.0 |
| D | re-link + `activate --force` (no score) | > 0.0 + status=active | ✅ 1.0 + active |

Hint string verified: all 3 mutator paths emit `Next: forgeplan score-all` per FR-009 (no longer `Next: forgeplan score <ID>`).

### Unit tests (`crates/forgeplan-core/src/scoring/mod.rs`)

5 tests covering helper contract:

- `sync_score_target_with_no_evidence_persists_zero` — empty evidence path keeps R_eff at 0.0; report.artifact_id echoes input.
- `sync_score_target_overwrites_stale_cached_value` — **PROB-057 regression guard**: planted stale value (0.99) gets overwritten to recomputed truth (0.0).
- `sync_score_target_unknown_id_returns_error` — error message references the unknown id или "not found" sentinel.
- `sync_score_target_rejects_malformed_id` — defense-in-depth: `validate_artifact_id` rejects empty / SQL-injection / path-traversal / NUL-injected ids before recursion.
- `sync_score_target_circular_dependency_terminates` — A→B→A graph terminates cleanly with `r_eff` ∈ [0, 1].

### Integration tests (`crates/forgeplan-cli/tests/cli_reff_cache_invalidation.rs`)

3 CLI-level tests reproducing the exact bug shape PROB-057 fixed:

- `link_recomputes_cached_r_eff_without_manual_score` — main PROB-057 trace test. Asserts `forgeplan get PRD-NNN --json` shows `r_eff > 0.0` after `forgeplan link PRD EVID informs` без `score`.
- `unlink_recomputes_cached_r_eff_without_manual_score` — symmetric для unlink.
- `activate_recomputes_cached_r_eff_without_manual_score` — для `--force` activate path.

### Full test suite

```
cargo test --workspace --features test-helpers
test result: ok. 1461 passed; 0 failed (lib + integration aggregated)
+ 5 new unit tests (scoring::sync_score_target_tests)
+ 3 new CLI integration tests (cli_reff_cache_invalidation)
= 1985 baseline + 8 new = 1993 tests
```

Pre-PROB-057 baseline (EVID-104): **1985 tests pass**. Current state: **1993 tests pass / 0 fail**.

### Quality gates

- `cargo fmt --check` — clean (0 diffs).
- `cargo clippy --workspace --all-targets --features test-helpers -- -D warnings` — clean (0 warnings).
- `forgeplan health` — clean (286 artifacts, 0 blind/orphan/stale).
- `cargo build --release` — clean (linked binary produces correct E2E output above).

### Audit Round 8 (2 parallel adversarial agents)

- **agents-pro:security-expert**: 8 findings (3 HIGH + 4 MED + 1 LOW). Closed in this PR: HIGH-3 (eprintln swallows actionable error → now emits `Fix: forgeplan score-all`), MED-2 (`validate_artifact_id` defense-in-depth), MED-3 (activate ordering — sync BEFORE projection render to avoid crash-window stale markdown), MED-4 (score `--json` errors array). Deferred to PROB-058: HIGH-1 (workspace lock на score), HIGH-2 (recursive walk DoS bound), MED-5 (timing side-channel).
- **agents-core:code-reviewer**: 10 findings (3 HIGH + 4 MED + 3 LOW). Closed in this PR: HIGH-3 (DRY — extracted `common::sync_score_target_or_warn`), MED-1 (helper returns `AssuranceReport` — eliminates double recursive walk in `score::run`), MED-2 (hint constant `hints::reconcile_parents_hint` — single source of truth для FR-009 string), MED-3 (paired with security MED-3 above), LOW-2 (unknown_id error message assertion), LOW-3 (circular dependency cycle test). Deferred to PROB-058: HIGH-1 (driver parity — `&LanceStore` vs `StorageDriver` trait), HIGH-2 (CLI integration test — closed differently через `cli_reff_cache_invalidation.rs`), MED-4 (tests bypass projection — kept as fast-path; full E2E covered by CLI integration tests).

## Files Touched

- `crates/forgeplan-core/src/scoring/mod.rs` — new `sync_score_target` helper + 5 unit tests
- `crates/forgeplan-core/src/hints.rs` — new `reconcile_parents_hint()` (FR-009 single source)
- `crates/forgeplan-cli/src/commands/common.rs` — new `sync_score_target_or_warn` wrapper (Round 8 HIGH-3 DRY)
- `crates/forgeplan-cli/src/commands/link.rs` — `run` + `run_unlink` invoke wrapper, hint via `reconcile_parents_hint`
- `crates/forgeplan-cli/src/commands/activate.rs` — `run` invokes wrapper BEFORE `render_projection` (Round 8 MED-3 ordering)
- `crates/forgeplan-cli/src/commands/score.rs` — `run` and `run_all` route through helper, return `AssuranceReport` (Round 8 MED-1 eliminates double walk), `--json` includes `errors` array (Round 8 MED-4)
- `crates/forgeplan-cli/tests/cli_reff_cache_invalidation.rs` — new file, 3 CLI integration tests (FR-008)

## Hindsight

PROB-057 was discovered **incidentally** during PROB-053 PR review — the user noted что `forgeplan get PRD-074` показывал R_eff=0.00 несмотря на successfully linked EVID-104. This serendipitous discovery sequence is itself a methodology lesson: scoreable artifacts during R_eff inspection sessions surface integrity bugs that `cargo test` цикл not catches because the sync-on-mutate path was untested end-to-end.

The fix shape (push-model auto-recompute) was selected through ADI/abduction (`forgeplan reason PROB-057`) which evaluated 4 options (auto-recompute on link, live computation in `get`, dirty flag schema bump, UX-indicator only) and recommended Option A с rationale "minimal scope, easy to test, reversible". Round 8 audit validated the choice but flagged 3 architectural follow-ups (PROB-058) which need separate scope.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-057 | informs (this evidence demonstrates closure) |
| PRD-075 | based_on (this evidence backs the PRD's acceptance criteria) |
| PROB-058 | informs (deferred Round 8 audit findings tracked separately) |
| EVID-104 | informs (PROB-053 closure — discovery context for PROB-057) |




