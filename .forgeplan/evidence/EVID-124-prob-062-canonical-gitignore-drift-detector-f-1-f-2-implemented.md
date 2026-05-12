---
depth: tactical
id: EVID-124
kind: evidence
links:
- target: PROB-062
  relation: informs
status: active
title: 'PROB-062: canonical .gitignore + drift detector — F-1 + F-2 implemented'
---

## Summary

PROB-062 (canonical `.gitignore` management + health drift detection) implemented across CLI init and core health surfaces. Scope covers F-1 (`forgeplan init` writes/refreshes marker-bounded section) and F-2 (`forgeplan health` advisory drift detector). F-3..F-5 (config.yaml split, migration command) explicitly out of scope for this evidence — separate sprint.

## Implementation

- `crates/forgeplan-cli/src/commands/init.rs`:
  - `ensure_canonical_gitignore_section()` writes root `.gitignore` with marker pair `# === forgeplan workspace runtime state (managed by \`forgeplan init\`) ===` / `# === end forgeplan section ===`.
  - `rewrite_gitignore()` pure helper covers three cases: empty file → write block; no markers → append with separator; markers present → replace block contents.
  - Wired into both `init_with_rollback` (fresh install) and `refresh_existing_workspace` (`--force` path).
  - Canonical patterns: `.forgeplan/lance/`, `.forgeplan/.fastembed_cache/`, `.forgeplan/session.yaml`, `.forgeplan/state/`, `.forgeplan/trash/`, `.forgeplan/logs/`, `.forgeplan/locks/`.

- `crates/forgeplan-core/src/health/mod.rs`:
  - `pub struct GitignoreDrift { path, reason }` (serde::Serialize).
  - `pub fn detect_gitignore_drift(workspace_root)` invokes `git -C <root> ls-files -- .forgeplan` (bounded scope, no full-repo walk) and matches each tracked path against `GITIGNORE_DRIFT_PATTERNS` table.
  - Silent on no-git / no-repo / subprocess failure — advisory contract.
  - `HealthReport.gitignore_drift: Vec<GitignoreDrift>` populated only by `health_report_with_phase` (which knows workspace path).
  - **Advisory by design**: NOT folded into verdict aggregator. Same class as PROB-063 phase mismatches.

- `crates/forgeplan-cli/src/commands/health.rs`: text + JSON output surface (`gitignore_drift` key).
- `crates/forgeplan-mcp/src/server.rs`: MCP `forgeplan_health` JSON response surface.

## Test Results

```
cargo test -p forgeplan-core --lib health::tests::detect_gitignore_drift
  test detect_gitignore_drift_no_forgeplan_dir_returns_empty ... ok
  test detect_gitignore_drift_no_git_repo_returns_empty ... ok
  test detect_gitignore_drift_ignores_tracked_artifact_bodies ... ok
  test detect_gitignore_drift_flags_tracked_lance_files ... ok
  test result: ok. 4 passed; 0 failed

cargo test -p forgeplan --lib gitignore
  test commands::init::gitignore_tests::rewrite_gitignore_empty_input_writes_block_as_is ... ok
  test commands::init::gitignore_tests::rewrite_gitignore_appends_with_blank_line_when_missing ... ok
  test commands::init::gitignore_tests::canonical_block_contains_markers_and_body ... ok
  test commands::init::gitignore_tests::ensure_canonical_gitignore_section_creates_when_missing ... ok
  test commands::init::gitignore_tests::ensure_canonical_gitignore_section_updates_existing_marker_block ... ok
  test commands::init::gitignore_tests::ensure_canonical_gitignore_section_preserves_user_rules ... ok
  test commands::init::gitignore_tests::ensure_canonical_gitignore_section_idempotent ... ok
  test result: ok. 7 passed; 0 failed

cargo test -p forgeplan --test cli_gitignore
  test init_creates_canonical_gitignore ... ok
  test init_preserves_existing_gitignore_rules ... ok
  test health_reports_gitignore_drift_when_present ... ok
  test result: ok. 3 passed; 0 failed
```

Pipeline gate:
- `cargo fmt --check` → 0 diffs
- `cargo clippy --workspace --all-targets -- -D warnings` → 0 warnings
- Full `cargo test --workspace`: 1674 lib tests pass; 2 pre-existing concurrent-flakes unrelated to this change (`commands::mcp::tests::which_on_path_finds_fake_binary` and `prob_060_stress_test::stress_test_property_loop_seeds` — both pass when run in isolation).

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Reproducibility

End-to-end manual verification:
1. Create a temp dir, run `git init`, then `forgeplan init -y` — observe `.gitignore` is created with the canonical block.
2. Add a pre-existing `.gitignore` with user rules before init — re-run init, user rules preserved and managed block appended.
3. Force-add `.forgeplan/lance/leaked.lance` and `.forgeplan/session.yaml` to git index; run `forgeplan health --json` — `gitignore_drift` array surfaces both with correct `reason` strings.
4. Verdict remains `empty` despite drift entries — advisory contract verified.



