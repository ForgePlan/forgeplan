---
depth: tactical
id: EVID-092
kind: evidence
links:
- target: PROB-047
  relation: informs
status: active
title: PROB-047 mitigation 1 — scan-import path blacklist verified
---

# EVID-092: PROB-047 mitigation 1 — scan-import path blacklist verified

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-28 |
| Valid Until | 2027-04-28 |
| Target | PROB-047 (`scan-import false-positive — classifies docs/guides/instructions as PRDs`) |

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Measurement

**Code change**: `crates/forgeplan-core/src/scan/detect.rs` + `import.rs`
(commit `aa2ed25` on branch `chore/post-phase6-cleanup-prob047-mitigation`).

New API:
- `is_doc_path(relative_path: &Path) -> bool` — blacklist for `docs/`,
  `marketplace/`, и root-level meta files (CLAUDE.md, AGENTS.md, README.md,
  CHANGELOG.md, CONTRIBUTING.md, TODO.md, ROADMAP.md, plus `.ru.md` localized
  variants).
- `detect_kind_with_path(filename, relative_path, content)` — path-aware
  variant that suppresses Tier 3 (content heuristic) only. Tier 1
  (frontmatter `kind:` field) и Tier 2 (filename pattern PRD-XXX/RFC-XXX/…)
  остаются authoritative — explicit signals всегда побеждают.
- `scan::import::scan_and_import_inner` switched to path-aware variant.

**Verification matrix** (in `scan::detect::tests`):

| Scenario | Expected | Actual | Tier used |
|----------|----------|--------|-----------|
| `docs/methodology/FORGEPLAN-GUIDE.md` + Goals/Problem headings, no frontmatter | None (no false-positive) | None | suppressed |
| `CLAUDE.md` + Problem heading, no frontmatter | None | None | suppressed |
| `docs/PRD-099-arch.md` + filename pattern, no frontmatter | PRD (filename precedence) | PRD | Filename |
| `docs/architecture.md` + `kind: prd` frontmatter | PRD (explicit opt-in) | PRD | Frontmatter |
| `.forgeplan/prds/PRD-099.md` + filename + content | PRD (real artifact location) | PRD | Filename |

**Backward compatibility**:
- `detect_kind(filename, content)` retained as wrapper passing `None` for
  path — все 15 existing tests pass без изменений.
- `is_doc_path` blacklist conservative: sub-crate `crates/foo/CHANGELOG.md`
  НЕ blacklisted (only root-level), `notes.md` без specific stem НЕ
  blacklisted (false-positive risk minimised).

## Result

**Test counts**:
- `scan::detect` module tests: was 15, now 26 (added 11 tests for
  is_doc_path coverage + path-aware Tier 3 suppression + Tier 1/2
  precedence under docs).
- Workspace-wide lib tests: 1400 passed, 0 failed (was 1389 на dev
  pre-commit aa2ed25, +11 new tests).
- Workspace gate: `cargo fmt -- --check` clean, `cargo check` 0 warnings,
  `cargo clippy --workspace --all-targets -- -D warnings` 0 errors.

**Test command**:

```bash
cargo test --package forgeplan-core --lib detect:: 2>&1 | tail -30
# 26 tests, all OK, 1302 filtered (other modules)

cargo test --workspace --lib 2>&1 | grep "^test result"
# every binary OK, total 1400 passed, 0 failed
```

## Interpretation

**Mitigation 1 closes 100% of the original PROB-047 trigger pattern**.
The 5 false-positive sources listed in PROB-047 body — `FORGEPLAN-GUIDE.md`,
`FORGEPLAN-GUIDE.ru.md`, `BROWNFIELD-ORCHESTRATOR-HANDOFF-2026-04-21.ru.md`,
`CLAUDE.md`, `SPEC-SCHEMA.md` — все classified ИСКЛЮЧИТЕЛЬНО через Tier 3
(content heuristic). После guard'а Tier 3 suppress'ится для всех paths
which match blacklist; результат — None (artifact not created).

**Что НЕ закрыто (4 mitigations remain in PROB-047 backlog)**:
1. ✅ Mitigation 1 — filename + path hybrid heuristic (THIS evidence).
2. ⏳ Mitigation 2 — `kind:` frontmatter precedence (already implemented
   in current code as Tier 1, just not formalized in PROB-047 wording).
3. ⏳ Mitigation 3 — scan-import default `--dry-run` + report, opt-in
   `--apply` flag (UX change, separate PRD).
4. ⏳ Mitigation 4 — content_hash-based idempotency (повторный run с тем
   же source updates existing instead of duplicating).
5. ⏳ Mitigation 5 — brownfield test fixtures (typical guides → assert
   scan-import создаёт 0 PRDs).

Mitigation 1 alone is sufficient для closure главных recurring symptoms,
but PROB-047 stays `active` until 4 remaining mitigations are addressed
in Phase 7+ sprint.

## Congruence Level Justification

**CL3 (same-context measurement)**. The evidence измеряет именно тот код
что упомянут в PROB-047 root cause hypothesis ("scan-import классификатор
смотрит на: filename pattern, markdown headings, YAML frontmatter `kind:`
field"). Tests are unit tests в том же module (`scan::detect`) что и
implementation. Workspace gate ran on the same commit. No simulation, no
proxy, no extrapolation — direct measurement of fix effectiveness.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PROB-047 | supports (mitigation 1 reduces false-positive rate to 0% for original 5 trigger patterns) |
| PRD-058 | informs (scan-import is the affected feature) |
| ADR-003 | informs (markdown source of truth — fix preserves invariant) |


