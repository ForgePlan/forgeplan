---
depth: tactical
id: EVID-078
kind: evidence
links:
- target: PRD-058
  relation: supports
status: draft
title: PRD-058 scan-import fix — CRITICAL double-FM closed + 3 HIGH audit fixes, 40 scan tests
---

# EVID-078: PRD-058 scan-import fix — CRITICAL double-FM closed + 3 HIGH audit fixes, 40 scan tests

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-19 |
| Valid Until | 2026-07-19 |
| Target | PRD-058 |

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

End-to-end validation of PRD-058 scan-import brownfield migration fix
on branch `fix/prob-scan-import-bugs` in Forgeplan workspace.

Method:
1. `cargo test -p forgeplan-core --lib scan::` — 40 tests pass
2. `cargo clippy --workspace --all-targets -- -D warnings` — clean
3. `cargo fmt --all --check` — clean
4. `cargo test --workspace` — 1405 tests pass
5. Real binary dogfood on fresh workspace with real Obsidian-format
   ADR file (`type: adr, status: accepted` frontmatter + body)
6. Two-agent adversarial audit (rust-pro + security-expert) on initial
   implementation — caught 1 CRITICAL + 3 HIGH + 3 MEDIUM. Every
   finding addressed before commit.

## Result

**Tests**: 40 passing scan-module tests (was 27 before PRD-058), 1405
total workspace tests / 0 fail. Delta: +8 status_map unit tests + 6
new AC-guard integration tests (projection creation, status mapping
accepted→active / rejected→superseded, unknown-status warning,
no-frontmatter default, idempotent second run).

**All 3 Telegram bug report bugs closed** (validated via `cargo
run --release forgeplan` on fresh workspace):

1. **Bug #2 (root cause)**: `.md` projection file now created at
   `.forgeplan/adrs/ADR-001-use-postgres.md` after scan-import. Fixed
   in `scan::import::maybe_write_projection` via `render_projection_record`.
2. **Bug #1**: `forgeplan get ADR-001` returns original body including
   `## Context` / `## Decision` sections (not empty template). Fixed
   via proper body extraction + preservation.
3. **Bug #3**: frontmatter `status: accepted` now maps to `active` via
   `status_map::map_external_status`. Unknown statuses (`wip`, etc.)
   default to `draft` + emit fail-loud warning surfaced in CLI output.

**`forgeplan reindex` round-trip**: `0 removed` on fresh scan-import
workspace. The root-cause Telegram symptom (reindex deleting all
imported artifacts) is no longer reproducible.

**Audit findings addressed in implementation**:
- **CRITICAL (rust-pro M-1)**: Double frontmatter — `file.content`
  passed as body into `render_projection_with_body` which prepended
  its OWN frontmatter. Caught by rust-pro via `eprintln` injection,
  not by my unit test (which used `.contains(substring)` that happily
  matched in duplicated content). Fixed: parse frontmatter once,
  split body, pass only body part. Empirically verified: projection
  file has exactly one `---…---` block.
- **HIGH (rust-pro M-2)**: Tags written to DB but not to projection
  file → next reindex (files-first per ADR-003) would null the tags
  column. Fixed: switched from `render_projection_with_body` to
  `render_projection_record` which carries tags through.
- **HIGH (rust-pro M-3)**: Skipped-branch never healed missing
  projections. Fixed: on `ImportStatus::Skipped`, if `.md` doesn't
  exist, write it. Closes partial-import resumption gap.
- **HIGH (rust-pro M-4)**: CLI (`scan_import.rs` + `init.rs`)
  silently dropped `entry.warnings`. Fixed: per-entry warning lines
  plus aggregate count in summary. PRD-058 R-2 fail-loud now enforced
  end-to-end.
- **MEDIUM (rust-pro M-5)**: Projection failure was best-effort —
  DB row remained but .md missing, violating ADR-003. Fixed: rollback
  DB insert (`store.delete_artifact`) when projection fails. ADR-003
  invariant holds after every scan-import exit path.
- **MEDIUM (rust-pro M-6 + security LOW #1)**: ID validation was by
  coincidence — relied on `store.create_artifact`'s check firing
  before the projection path-join. Made explicit via local
  `is_safe_artifact_id` in `resolve_artifact_id`. Rejects `..`, `/`,
  `\`, null bytes; a crafted frontmatter `id: ../../etc/passwd` now
  falls through to auto-generated sequential ID.
- **MEDIUM (rust-pro M-7)**: Deprecated the bare `scan_and_import`
  (ADR-003-non-compliant) with a clear `#[deprecated(since="0.25.0",
  note=…)]` attribute. Existing tests opt in via
  `#[allow(deprecated)]` at the test-module level. New callers are
  forced onto `scan_and_import_to_workspace`.

**Security audit** (separate agent): 0 CRITICAL / HIGH / MEDIUM.
Two LOW findings (status input unbounded allocation, log injection
via unknown status) — acceptable, upstream frontmatter parser already
size-bounds via the 64 KB limit, and `{other:?}` Debug formatting
escapes control chars.

## Interpretation

The Telegram brownfield bug report (33 Obsidian ADRs with
`status: accepted`) was a caskade from one root-cause: scan-import
only wrote to LanceDB, never to the filesystem. That violated
ADR-003 "Markdown primary, LanceDB derived", which made the next
`forgeplan reindex` purge every imported artifact as "orphan" (no
.md file).

The fix threads through three concerns end-to-end:
1. **FR-001** (projection write) closes ADR-003 violation.
2. **FR-002 + FR-003** (status mapping) preserves semantic fidelity
   — external tool vocabularies (Obsidian, MADR, ADR-tools) round-
   trip into the canonical Forgeplan lifecycle.
3. **FR-004** (warnings) keeps the contract fail-loud — unknown
   values surface, are not silently rounded.

The two-round audit (initial implementation + hotfix after findings)
demonstrates the same pattern established across PRD-055/056/057:
careful unit tests don't catch structural bugs that an adversarial
reviewer finds by running `eprintln` on the written file. Critical
for this class of changes (filesystem + DB consistency) where test
assertions using `.contains(substring)` happily pass with duplicated
content.

## Congruence Level Justification

CL3 (same-context): evidence runs against the exact branch under
review (`fix/prob-scan-import-bugs`) with the project's own test
suite and real release-profile binary. Dogfood uses the same brew
install pattern external users follow. Traceable in-repo: all code
paths named by file:line.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-058 | supports |
| ADR-003 | informs (PRD-058 enforces the invariant) |

