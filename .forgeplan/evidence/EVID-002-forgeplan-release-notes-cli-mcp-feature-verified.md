---
depth: tactical
id: EVID-002
kind: evidence
status: active
title: forgeplan release-notes CLI + MCP feature verified
---

# EVID-002: forgeplan release-notes CLI + MCP feature verified

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-05-12 |
| Valid Until | 2026-08-12 |
| Target | v0.31.0 sprint Wave 4 MAJOR-3 (manual CHANGELOG pain) |

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Context

Wave 4 of the v0.31.0 sprint cataloged "manual CHANGELOG reconstruction"
as a recurring pain point (MAJOR-3): every release branch required hand-
reading every artifact touched since the previous tag to bucket them
into Keep-a-Changelog sections. The work was repeatable, mechanical, and
high-friction — a textbook automation candidate.

Worker `w6-release-notes` implemented `forgeplan release-notes` (CLI
subcommand + `forgeplan_release_notes` MCP tool) closing that gap.

## What was built

1. **`forgeplan_core::release_notes`** — pure logic module:
   `Category` enum (Added / Fixed / Security / Changed / Internal),
   `classify()` mapping (kind + status + R_eff + security-link → section),
   markdown / text / JSON formatters, git-log walker with
   `--diff-filter=AM`, basename-to-id resolver (post-merge `KIND-NNN`
   shape **and** pre-merge slug shape per SPEC-005), R_eff quality gate
   (active OR `r_eff_score > 0`), `--draft` waiver.

2. **`forgeplan release-notes`** CLI subcommand —
   `[--since <ref>] [--until <ref>] [--output text|markdown|json]
   [--draft]`. Defaults: `since` = latest git tag from
   `git describe --tags --abbrev=0`, `until` = `HEAD`, `output` =
   `markdown`. Refs validated through the existing
   `forgeplan_core::git::validate_git_ref` (CWE-88 argument-injection
   guard).

3. **`forgeplan_release_notes` MCP tool** — same param surface, JSON
   payload identical to CLI `--output json`. Read-only tool annotation.

## Tests

- **Unit (forgeplan-core)** — 20 tests covering classification (every
  kind × status combination), quality gate (draft / active / score),
  Keep-a-Changelog formatters (markdown / text / json shape),
  basename id-extraction (pre-merge + post-merge filename shapes), and
  empty-state messaging. All PASS.
- **CLI integration (`crates/forgeplan-cli/tests/cli_release_notes.rs`)**
  — 6 tests building a self-contained git history in a tempdir:
  PRD-in-Added, JSON shape parse, quality gate filtering drafts,
  text-no-markdown-chars, invalid-output rejection, and
  `--upload-pack=…` argument-injection rejection. All PASS.
- **MCP smoke (`integration_full_coverage::c60_forgeplan_release_notes_smoke`)**
  — verifies the tool is reachable + returns parseable payload. PASS.

## Dogfood demonstration

Synthetic workspace at `/tmp/demo-relnotes` with v0.30.0 tag and a
v0.31.0-shaped artifact set:

```
$ forgeplan release-notes --since v0.30.0 --output markdown --draft
## [v0.30.0 → HEAD]
> Draft mode — quality gate disabled.

### Added
- Health Strict Flag (PRD-001, commit 4bb960b)

### Fixed
- Counter race condition in worktrees (PROB-001, EVID-001, commit 4bb960b)
- Init force scan data-loss (PROB-002, commit 4bb960b)
- Forgeplan guard target_phase enum overlap (PROB-003, commit 4bb960b)

### Changed
- Phase 4 ID assignment cleanup (RFC-001, commit 4bb960b)

### Internal
- PROB-067 atomic counter fix verified (EVID-001, commit 4bb960b)
```

Quality gate (without `--draft`) filters drafts that have neither
`status==active` nor `r_eff_score > 0`:

```
$ forgeplan release-notes --since v0.30.0 --output markdown
## [v0.30.0 → HEAD]
_No artifacts matched the requested range._
```

The PROB→Fixed entry with the attached EVID-001 demonstrates that the
incoming-relation walker correctly threads closing evidence into each
entry — matching the canonical PROB+EVID flow from CLAUDE.md.

Output saved to `/tmp/v031-changelog-draft.md`.

## Pipeline

- `cargo fmt -- --check` — 0 diffs.
- `cargo check --workspace` — 0 warnings.
- `cargo clippy --workspace --all-targets -- -D warnings` — 0 findings.
- `cargo test --workspace --lib` — all PASS (excluding pre-existing
  `commands::mcp::tests::which_on_path_finds_fake_binary` flake that
  also fails on `git stash` baseline → not regression-introduced).
- `cargo test -p forgeplan --test cli_release_notes` — 6/6 PASS.
- `cargo test -p forgeplan-mcp --test integration_full_coverage c60` — 1/1 PASS.
- `bash scripts/smoke-test.sh` — PASS (13 operations, 8 artifact kinds).

## Verdict

The CHANGELOG-from-artifacts pipeline is now a single command. The
feature self-uses at the next release (`forgeplan release-notes
--since v0.31.0` will produce the v0.32.0 draft) — dogfood-driven
priority delivered.


