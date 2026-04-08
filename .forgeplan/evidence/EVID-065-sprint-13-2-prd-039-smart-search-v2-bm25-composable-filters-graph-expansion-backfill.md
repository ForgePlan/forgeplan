---
depth: tactical
id: EVID-065
kind: evidence
links:
- target: PRD-039
  relation: informs
- target: EPIC-003
  relation: informs
status: active
title: Sprint 13.2 PRD-039 Smart Search v2 — BM25 + Composable Filters + Graph Expansion (backfill)
---

# EVID-065: Sprint 13.2 PRD-039 Smart Search v2 Implementation Evidence

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Summary

**Backfilled during final v0.17.0 release audit (2026-04-08)** — Sprint 13.2
PRD-039 Smart Search v2 was shipped per EPIC-003 roadmap and merged into
`release/v0.17.0`, but the closeout never created the evidence artifact
that would have linked PRD-039 → evidence. Final-docs auditor flagged
this as a release blocker. This backfill evidence documents what shipped,
verified against the actual merged code on `release/v0.17.0` and the
running release binary.

## What shipped (verified on release/v0.17.0 binary)

Sprint 13.2 delivered Smart Search v2 per PRD-039:

### FR-001 — BM25 relevance scoring
Replaced the prior substring-match scorer in
`crates/forgeplan-core/src/db/store.rs::search` with BM25 ranking.
Each result gains a `bm25_score: f64` field. Verified via final-e2e
cross-sprint check: `forgeplan search "auth"` returns graceful
`No results` on empty workspace and ranked results on populated.

### FR-002 — Composable ArtifactFilter DSL
`ArtifactFilter` struct in `crates/forgeplan-core/src/db/store.rs`
gains composable fields: `kind`, `status`, `depth`, `tags`,
`since: Option<NaiveDate>`, `with_evidence`, `no_evidence`. The store
builds a Lance filter expression from whichever fields are `Some`.

### FR-003 — 1-hop graph neighbor expansion
New `SmartSearchResult.expanded_from: Option<String>` tracks
artifacts brought into the result set by graph expansion. CLI flag
`--no-expand` skips neighbor expansion. Default behavior expands
once across `informs`/`based_on`/`refines` relations.

### CLI flags added to `forgeplan search`
- `--status <s>` — filter by status
- `--depth <d>` — filter by depth
- `--with-evidence` / `--no-evidence` — filter by evidence presence
- `--since YYYY-MM-DD` — filter by creation/update date
- `--no-expand` — skip graph expansion

### MCP `search` tool extended
`SearchParams` struct extended to mirror the CLI flags. AI agents can
now filter search results without client-side post-processing.

## Test coverage

All tests ship with the commits that introduced them and remain green
on `release/v0.17.0`:
- `forgeplan-core` BM25 scoring unit tests (in `db/store.rs`)
- `forgeplan-core` filter DSL unit tests (compositional cases)
- `forgeplan-cli` integration tests for search command + flags
- `forgeplan-mcp` param validation tests for extended `SearchParams`

Verified on `release/v0.17.0` by final-release audit team:
- `cargo test --workspace`: 1109 tests pass (includes all Sprint 13.2 tests)
- `tests/e2e/sprint-13.7-regression.sh`: includes a smart search check
  inherited from Sprint 13.2 (`✓ 13.2 smart search`)
- Cross-sprint final-e2e scenario: `$BIN search "auth" --no-expand`
  returns graceful no-results + hint on sparse workspace

## Why this evidence is backfill (not suspicious)

Sprint 13.2 was executed and merged (PR before #154), but the closeout
step that creates EVID-XXX and runs `forgeplan activate PRD-039` was
skipped at the time. The final release docs audit caught this as a
missing artifact and the team-lead created this evidence retroactively
to satisfy:

1. PRD-039 needs non-zero R_eff before `forgeplan activate`
2. EPIC-003 progress tracking expects each child PRD to have evidence
3. NOTE-044 Sprint Checklist Framework Phase 6 (Closeout) mandates
   "EVID artifact with structured fields" for every shipped sprint

The backfill documents what's **actually in the merged code** on
`release/v0.17.0`, not aspirational work. Independent verification came
from final-e2e agent during the release audit.

## Lesson for NOTE-044

Add to Phase 6 Closeout checklist (already present but strengthened):
> [ ] **Verify EVID exists for current sprint BEFORE opening the next
>      sprint branch**. Missing EVIDs slip through review if the next
>      sprint's velocity masks the gap.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PRD-039 | informs (this evidence supports FR-001..FR-003) |
| EPIC-003 | informs (Sprint 13 v0.17.0 series) |
| EVID-058..064 | sibling (Sprint 13.1, 13.1.5, 13.3, 13.4, 13.5, 13.6, 13.7 evidence) |
| NOTE-044 | process (checklist that would have prevented this gap) |
| NOTE-045 | context (Sprint 13.7 retrospective that triggered the audit) |


