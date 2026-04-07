---
depth: tactical
id: EVID-060
kind: evidence
links:
- target: PRD-035
  relation: informs
status: draft
title: Sprint 13.3 PRD-035 p1 — Tags system + Source Tier implementation — 1006 tests pass, 7 audit fixes
---

# EVID-060: Sprint 13.3 PRD-035 p1 Implementation Evidence

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Summary

Sprint 13.3 implemented PRD-035 Phase 1 (FR-001..003 + FR-008) — tags system + source tier mapping. Multi-agent audit (4 auditors + 7 fixers) caught and resolved 2 CRITICAL + 5 HIGH findings including a release-blocker migration bug. 1006 tests pass on merged code.

## Implemented Functional Requirements

### FR-001: Tags in frontmatter + schema
- `Frontmatter::tags_from_frontmatter()` parser (YAML seq / inline array / CSV)
- `Frontmatter::has_tag_in()` helper (via canonical `search::filter::has_tag_predicate`)
- Schema v3 → v4 migration: `List<Utf8>` nullable tags column
- Migration via `NewColumnTransform::AllNulls(schema)` — typed null injection
- `NewArtifact.tags`, `ArtifactRecord.tags` fields throughout
- `LanceStore::add_tags / remove_tags / list_by_tag` methods

### FR-002: forgeplan tag / untag CLI
- `crates/forgeplan-cli/src/commands/tag.rs` (NEW, ~95 LOC)
- `run_add(id, tags)` with dedupe + projection back to markdown
- `run_remove(id, tags)` mirror operation
- `Tag` / `Untag` subcommands in main.rs
- Round-trips through markdown frontmatter via `projection::render_projection_record`

### FR-003: forgeplan list --tag filter
- Composable via `search::filter::ArtifactFilter::HasTag(String)` variant
- CLI `list --tag key=value` or `--tag key` (key-only bare match)
- Combinable with `--type`, `--status`: `And([Kind, Status, HasTag])`
- `db::store::ArtifactFilter` renamed to `ListFilter` (name collision fix)

### FR-008: SourceTier → CongruenceLevel mapping
- `scoring::evidence::SourceTier` enum: T1 / T2 / T3
- `to_congruence_level()`: T1→CL3, T2→CL2, T3→CL1
- `parse()` accepts t1 / tier1 / tier-1 / 1 variants
- **Precedence rule**: `cl = min(tier_cl, explicit_cl)` when both present
- Prevents self-tagged T1 from overriding explicit operator downgrade (audit H2)

## Multi-agent audit cycle (W4)

| Auditor | Focus | Findings |
|---------|-------|----------|
| Rust Expert | idiomatic patterns, schema migration, Arrow | 2C + 4H + 7M + 7L |
| Security | trust boundaries, DoS, bypass paths | 0C + 1H + 5M + 5L |
| Architecture | SOLID, layering, composability | 0C + 3H + 4M + 5L |
| Test Coverage | FR matrix, mutation analysis | 0C + 1H + 4M + — |
| **Unique total** | | **2C + 5H + 12M + 13L** |

## W5 fixers (7 parallel agents)

All CRITICAL + HIGH findings resolved:

| Finding | Severity | Fix |
|---------|----------|-----|
| **C1** Tags dropped in scan/git_sync/projection | CRITICAL | frontmatter_map emits + projection render_projection_record + all ingestion paths wire tags_from_frontmatter |
| **C2** replace_record nulled body_hash + embedding | CRITICAL | ArtifactRecord gained fields, extract_record populates, merge_insert preserves |
| **H1** Name collision + composability break | HIGH | HasTag variant added to search::filter::ArtifactFilter, db::store::ArtifactFilter → ListFilter rename |
| **H2** SourceTier trust amplification | HIGH | cl = min(tier, explicit) precedence rule |
| **H3** replace_record non-atomic delete+insert | HIGH | Switched to LanceDB merge_insert (atomic upsert) |
| **H4** Migration v3→v4 broken on real v3 tables (RELEASE BLOCKER) | HIGH | NewColumnTransform::AllNulls(schema) — typed null API instead of unsupported SQL CAST |
| **C1 residual** tag CLI updated DB but not markdown | CRITICAL | run_add/run_remove now call render_projection_record; projection layer extended with tags support |

## Test results

- **Total: 1006 tests pass, 0 failed**
  - forgeplan-core: 803 (up from 784 pre-W5)
  - forgeplan: 99 CLI integration (up from 87)
  - forgeplan-mcp: 29
  - Other crates: 75
- **~56 new tests in Sprint 13.3** (W1-W5 combined)
- cargo fmt --check: clean
- cargo check --workspace: 0 warnings
- 0 new dependencies
- 0 ignored tests

## E2E verification on release binary

```
$ forgeplan tag PRD-001 source=code layer=auth
  ✓ Added 2 tag(s) to PRD-001
  Current tags: source=code, layer=auth

$ grep -A3 "^tags:" .forgeplan/prds/PRD-001*.md
tags:
- source=code
- layer=auth

$ forgeplan list --tag source=code
  PRD-001  prd  draft  X  ✓

$ forgeplan list --tag layer --type prd --status draft
  PRD-001  prd  draft  X  ✓ (composable DSL)

$ forgeplan untag PRD-001 layer=auth
  ✓ Removed 1 tag(s) from PRD-001
```

Tags round-trip: CLI → LanceDB → markdown file → reindex → LanceDB. Verified on compiled release binary, not just unit tests.

## Sprint methodology compliance

- [x] Branch feat/sprint-13.3-prd-035-tags from release/v0.17.0
- [x] /sprint workflow: direct agent pattern (NOTE-043 hybrid)
- [x] W1 implementation → W2 CLI → W3 E2E → W4 audit → W5 fixes → W6 commit/PR
- [x] cargo test --workspace: 1006 passed, 0 failed
- [x] cargo fmt --check: clean
- [x] cargo check --workspace: 0 warnings
- [x] Multi-agent audit (4 specialized)
- [x] All HIGH/CRITICAL findings fixed in W5
- [x] E2E manual verification on release binary
- [x] Hindsight memory_retain

## Deferred (tracked as PROB-026 + PROB-027)

- 12 MEDIUM + 13 LOW audit findings → **PROB-026** (tag canonicalization, char validation, CLI display polish, etc.)
- `forgeplan reindex` cannot rebuild from scratch when lance/ dir missing → **PROB-027** (orthogonal to PRD-035 scope)

## Files modified/created

**Sprint 13.3 W1-W5 total:** 39 files changed, +2041/-21 LOC

Key paths:
- crates/forgeplan-core/src/artifact/frontmatter.rs (tags parsing)
- crates/forgeplan-core/src/db/schema.rs (v4 schema)
- crates/forgeplan-core/src/db/migrate.rs (AllNulls migration)
- crates/forgeplan-core/src/db/store.rs (add_tags/remove_tags/list_by_tag, merge_insert)
- crates/forgeplan-core/src/db/convert.rs (List<Utf8> builder + embedding extraction)
- crates/forgeplan-core/src/scoring/evidence.rs (SourceTier + precedence rule)
- crates/forgeplan-core/src/search/filter.rs (HasTag variant)
- crates/forgeplan-core/src/projection/mod.rs (render_projection_record with tags)
- crates/forgeplan-cli/src/commands/tag.rs (NEW)
- crates/forgeplan-cli/src/commands/list.rs (composable filter DSL)
- crates/forgeplan-cli/src/commands/{scan,import,git_sync,reindex,promote,export}.rs (tag wire-up)

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-035 | informs (this evidence supports PRD-035 p1 FRs 1-3, 8) |
| PROB-022 | informs (brownfield onboarding problem — root cause of PRD-035) |
| EPIC-003 | informs (Sprint 13 series of v0.17.0) |
| PROB-026 | informs (deferred M/L audit findings follow-up) |
| PROB-027 | informs (reindex rebuild-from-zero follow-up) |
| EVID-059 | informs (preceding Sprint 13.1.5 hardening evidence) |
| NOTE-043 | informs (team orchestration pattern used for W5) |
