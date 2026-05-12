---
depth: tactical
id: EVID-122
kind: evidence
links:
- target: PROB-068
  relation: informs
status: active
title: PROB-068 init/scan-import data-loss closure
---

# EVID: PROB-068 init/scan-import data-loss closure

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Context

Wave 6B sprint, 2026-05-12 — w6-prob-068-fix worker. Branch
`fix/prob-068-init-dataloss` off dev 20c2c35.

PROB-068 documented two data-loss vectors against existing artifacts:

1. `forgeplan init --force --scan` in a populated workspace blew away
   markdown artifact bodies (kept frontmatter, dropped narrative).
2. `forgeplan scan-import` round-tripped existing frontmatter through
   LanceDB and reverse-mutated 83 artifact frontmatters — dropped
   `links:` sections + injected `author: scan-import`.

Both were 100% reproducible and required `git restore .forgeplan/` to
recover. Severity HIGH (CWE-664 / CWE-693).

## What changed

Combo A + B + C from the locked design landed together:

- **Option A (init --force additive)** — `--force` no longer calls
  `safe_remove_workspace`. The new flow `refresh_existing_workspace`
  ensures every artifact subdir exists, rewrites `config.yaml` with the
  old one preserved as `config.yaml.bak-<ts>`, and re-initializes the
  LanceDB index in place. Artifact `.md` bodies are never touched.

- **Option B (scan-import union-merge)** — `process_detected_file_inner`
  now parses the source frontmatter once and extracts `author` +
  `links` via two new helpers in `artifact/frontmatter.rs`
  (`author_from_frontmatter`, `links_from_frontmatter`). The harvested
  author flows into `NewArtifact.author` (defaulting to `scan-import`
  only when the source had no author); the harvested links are inserted
  through `store.add_relation` post-create and passed through to
  `maybe_write_projection`. Both code paths — fresh import and
  Skipped+heal — use the same union-merge.

- **Option C (auto-backup)** — `--force` invokes `create_force_backup`
  before any refresh and writes `.forgeplan-backup-<UTC-timestamp>/`
  containing every `ARTIFACT_DIRS` subdirectory that exists. A new
  `--no-backup` flag opts out. Backup failures abort the operation so
  the caller never silently loses data.

## Verification

Regression suite `crates/forgeplan-cli/tests/cli_init_safety.rs` (5
tests, all PASS in 5.12s):

1. `init_force_preserves_existing_artifact_bodies` — populated PRD-099
   with body + `author: human-author` + `links:` + custom field
   survives `init -y --force --no-backup` byte-for-byte.
2. `init_force_auto_backups_existing_artifacts` — `init -y --force`
   without `--no-backup` produces `.forgeplan-backup-*/prds/PRD-099-*.md`
   with the original body intact.
3. `init_force_no_backup_flag_skips_backup` — explicit `--no-backup`
   suppresses the `.forgeplan-backup-*` directory.
4. `scan_import_preserves_links_section` — `docs/PRD-150-feature.md` with
   two-target `links:` block survives scan-import; projection
   `.forgeplan/prds/PRD-150-*.md` still contains both `target:`/`relation:`
   pairs.
5. `scan_import_preserves_author_field` — `docs/ADR-099-original-author.md`
   with `author: human-author` lands in the projection with
   `author: human-author` (not `scan-import`).

Pipeline gate:

- `cargo fmt -- --check` — clean
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo test --workspace` — all suites PASS (core 1647 + integration
  groups, no failures, no regressions)
- `bash scripts/smoke-test.sh` — full E2E PASSED, 8 artifact kinds,
  13 operations including init + scan-import paths

## Limitations

- Backup performance on very large workspaces (1000+ artifacts): the
  implementation copies every `.md` file under `ARTIFACT_DIRS` with
  `tokio::fs::copy`. For dev (~309 artifacts) this is sub-second;
  workspaces an order of magnitude larger should still complete in a
  few seconds. If this becomes a bottleneck, a `tar`-stream variant is
  a follow-up optimization.
- The historical destructive `--force` semantics are gone; users who
  relied on "wipe everything" must now `git rm -rf .forgeplan && init`
  manually. Help text on `--force` advertises the new contract.
- Option B fixes the round-trip for files whose `links:` block matches
  the canonical `target:` + `relation:` shape. Free-form / alternative
  YAML shapes are silently dropped (`links_from_frontmatter` returns
  empty rather than guessing). Acceptable for the documented Forgeplan
  contract; out-of-shape blocks were never round-trip-safe.

## Acceptance criteria mapping (from PROB-068)

1. AC-1 — init --force preserves all artifact .md body content
   → covered by `init_force_preserves_existing_artifact_bodies`.
2. AC-2 — scan-import preserves `links:` + `author:` fields
   → covered by `scan_import_preserves_links_section` and
   `scan_import_preserves_author_field`.
3. AC-3 — regression tests verify links + bodies unchanged through
   `init --force --scan` cycle
   → tests #1 + #4 + #5 jointly cover the cycle.
4. AC-4 — CLI help text describes destructive vs non-destructive
   semantics — addressed in `main.rs` doc-comment on the `Init` command
   ("strictly additive — never overwrites existing artifact .md bodies").
5. AC-5 — auto-backup `.forgeplan-backup-XXX/` unless `--no-backup`
   → covered by `init_force_auto_backups_existing_artifacts` and
   `init_force_no_backup_flag_skips_backup`.

## Refs

- PROB-068 (this evidence's parent)
- ADR-003 (file-first invariant — re-asserted by Option A + B)
- PRD-058 (scan-import projection contract — extended)
- PRD-073 (file-first invariant compile-enforcement — same family)
- PROB-061 (reindex reset family — informs)

