---
depth: tactical
id: EVID-058
kind: evidence
links:
- target: PRD-019
  relation: informs
- target: PRD-043
  relation: informs
status: active
title: Sprint 13.1 PRD-043 Methodology Integrity — duplicate guard, stub gate, health detection, MCP warnings — 879 tests pass
---

---
id: EVID-058
title: "Sprint 13.1 PRD-043 Methodology Integrity — duplicate guard, stub gate, health detection, MCP warnings — 879 tests pass"
status: Draft
created: 2026-04-07
---

# EVID-058: Sprint 13.1 PRD-043 Implementation Evidence

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Summary

Sprint 13.1 implemented all 4 functional requirements of PRD-043 (Methodology Integrity) with full test coverage and post-audit fixes. Evidence collected from team-based execution with `/sprint` workflow (TeamCreate, 7 teammates across 4 waves).

## What was implemented

### FR-001 — Duplicate guard in `forgeplan new`
- File: `crates/forgeplan-cli/src/commands/new.rs` (+105 LOC)
- Pre-create check: smart_search by title against same-kind existing artifacts
- If similarity ≥ 0.7 (canonical Jaccard threshold) → cliclack confirm prompt
- Escape hatch: `--allow-duplicate` flag (renamed from `--force` per audit H-4)

### FR-002 — Health duplicate detection
- File: `crates/forgeplan-core/src/health/mod.rs` (+191 LOC)
- New: `find_duplicate_pairs(records, threshold) -> Vec<DuplicatePair>`
- New: `find_active_stubs(records) -> Vec<ActiveStub>` (added in W4 H-2 fix)
- HealthReport extended: `possible_duplicates`, `active_stubs`
- Display in `cli/commands/health.rs` (+54 LOC)

### FR-003 — Stub validation rule + activate gate
- File: `crates/forgeplan-core/src/validation/rules.rs` (+224 LOC)
- New: `check_stub(body, frontmatter) -> Option<String>` — counts template markers
- Bilingual marker set (Russian + English) per audit H-3
- Registered as `no-stub-content` Severity::Should
- File: `crates/forgeplan-core/src/lifecycle/mod.rs` (+127 LOC)
- Activate gate: hard-blocks stubs regardless of Severity
- Bypass: `--force` flag (intentional, separate from CLI --allow-duplicate)

### FR-004 — MCP duplicate warnings
- Files: `crates/forgeplan-mcp/src/server.rs` (+92 LOC), `types.rs` (+12 LOC)
- New: `DuplicateWarning` struct in types
- `forgeplan_new` returns warnings field (non-breaking, serde default)
- Artifact still created (MCP non-interactive); AI agent decides on warnings

### Canonical similarity (W4 C-1 fix)
- New: `crates/forgeplan-core/src/duplicate/` module
- Single canonical `title_similarity()` (Jaccard tokens, ≥3 chars)
- Pinned threshold: `DUPLICATE_SIMILARITY_THRESHOLD = 0.7`
- All 3 callsites (CLI, MCP, Health) call canonical fn — no more divergence

## Test results

- Total tests: **879 pass, 0 fail**
- New tests added in Sprint 13.1: ~40
  - 4 stub validation tests
  - 4 duplicate guard CLI tests
  - 4 health duplicate detection tests
  - 2 lifecycle stub block tests
  - 3 MCP warning tests
  - 11 integration tests in `tests/integrity_test.rs` (NEW)
  - English template marker test
  - find_active_stubs tests
- `cargo test --workspace`: ok
- `cargo fmt --check`: clean
- `cargo check --workspace`: 0 warnings

## Audit results

- 1 CRITICAL (C-1: divergent algorithms) → FIXED in W4
- 4 HIGH:
  - H-1 (off-by-one >0.8) → FIXED (W4)
  - H-2 (stub bypass via scan-import) → FIXED via find_active_stubs (W4)
  - H-3 (Russian-only markers) → FIXED with English markers (W4)
  - H-4 (--force overload) → FIXED via rename to --allow-duplicate (W4)
- 5 MEDIUM, 4 LOW → DEFERRED to PRD-044 (per audit recommendation)

## Files modified

```
crates/forgeplan-cli/src/commands/health.rs   (+54)
crates/forgeplan-cli/src/commands/new.rs      (+105/-7)
crates/forgeplan-cli/src/main.rs              (+9)
crates/forgeplan-core/src/duplicate/mod.rs    (NEW, ~80 LOC)
crates/forgeplan-core/src/health/mod.rs       (+191)
crates/forgeplan-core/src/lib.rs              (+1)
crates/forgeplan-core/src/lifecycle/mod.rs    (+127)
crates/forgeplan-core/src/validation/rules.rs (+224)
crates/forgeplan-mcp/src/server.rs            (+92)
crates/forgeplan-mcp/src/types.rs             (+12)
crates/forgeplan-core/tests/integrity_test.rs (NEW, 11 tests)
```

Total: ~895 LOC source + tests across 11 files.

## Sprint methodology compliance

- [x] Branch `feat/sprint-13.1-prd-043-integrity` from `release/v0.17.0` (NOT dev)
- [x] /sprint workflow: TeamCreate + 4 waves of teammates
- [x] Wave 1 → Wave 2 → Wave 3 → Wave 4 (audit fixes)
- [x] cargo test --workspace: 879 passed, 0 failed
- [x] cargo fmt --check: clean
- [x] cargo check --workspace: 0 warnings
- [x] Audit by code-reviewer agent (W3)
- [x] All HIGH/CRITICAL fixed in W4
- [x] No new dependencies (Cargo.toml unchanged)

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-043 | informs (this evidence supports it) |
| PROB-024 | informs (PRD-043 solves this problem) |
| EPIC-003 | informs (Sprint 13.1 of EPIC-003) |


