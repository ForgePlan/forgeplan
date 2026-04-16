---
depth: tactical
id: PROB-024
kind: problem
links:
- target: EPIC-003
  relation: informs
status: deprecated
title: Duplicate artifact creation — no semantic guard, stub activation possible
---

---
id: PROB-024
title: "Duplicate artifact creation — no semantic guard, stub activation possible"
status: Draft
created: 2026-04-07
depth: standard
context: "methodology-integrity"
parent_epic: EPIC-003
---

# PROB-024: Duplicate artifact creation — no semantic guard, stub activation possible

## Signal

Real incident (2026-04-07, Sprint 13 shaping session):

1. **Duplicate created:** I (Claude) ran `forgeplan new prd "FPF Knowledge Base — Vector Search via EmbedDriver"` and got PRD-042 — without realizing PRD-018 with title "FPF Knowledge Base — semantic search" already existed (active, R_eff=1.00).

2. **Stub activated:** PRD-018 was active with R_eff=1.00 but body contains only template placeholders (`Vision: ...`, `Problem: ...`, `[Actor] can [capability]`). It was never properly shaped — yet it passed activation gates.

3. **Search did not find it:** `forgeplan search "FPF semantic vector"` returns 0 results despite PRD-018 existing — because current keyword search uses substring grep which requires exact phrase matching.

The chain of failures: stub activated → semantic search broken → developer creates duplicate without noticing → validate/health pass → duplicate enters source of truth.

## Constraints

- MUST not break existing PRD-018 / PRD-042 flow (preserve lineage via supersede)
- MUST not require LLM for duplicate detection (works offline)
- MUST handle false positives gracefully (ask user, do not block silently)
- SHOULD work on empty workspace (init time, 0 artifacts)
- SHOULD detect both kinds of stubs: never-filled and template-only

## Optimization Targets

1. **Prevent duplicate creation** — `forgeplan new` should warn before creating semantically similar artifact
2. **Detect existing duplicates** — `forgeplan health` should surface duplicate candidates
3. **Block stub activation** — `forgeplan activate` should fail if body matches template patterns

## Observation Indicators (Anti-Goodhart)

- DO NOT optimize: "0 duplicates ever" — false positives will frustrate users
- MONITOR: rate of false-positive warnings (user dismisses guard) — if > 30%, threshold too aggressive
- MONITOR: number of stub artifacts in workspace — should trend to 0
- DO NOT optimize: "minimum similarity threshold" — too low = noise, too high = misses real dups

## Acceptance Criteria

1. **AC-1: Duplicate guard at creation time**
   - Given workspace has PRD-018 "FPF Knowledge Base — semantic search"
   - When I run `forgeplan new prd "FPF Knowledge Base — vector search"`
   - Then system shows: "⚠ Found 1 similar artifact: PRD-018 (similarity 87%)"
   - And asks: "Continue? [y/N/show]"

2. **AC-2: Health detects duplicates**
   - Given workspace has 2 artifacts with similarity > 80%
   - When I run `forgeplan health`
   - Then output includes "Possible duplicates (1)" section with both IDs

3. **AC-3: Stub activation blocked**
   - Given an artifact body contains 3+ template markers (`...`, `[Actor] can`, `{placeholder}`)
   - When I run `forgeplan activate <id>`
   - Then activation fails with "Artifact appears to be a stub — fill MUST sections first"

## Blast Radius

- `crates/forgeplan-cli/src/commands/new.rs` — add guard before creation
- `crates/forgeplan-core/src/health/mod.rs` — add duplicate check
- `crates/forgeplan-core/src/validation/rules.rs` — add stub detection rule
- `crates/forgeplan-core/src/lifecycle/transitions.rs` — block activation on stub
- All future `forgeplan new` commands affected — must communicate behaviour change

## Reversibility

**Medium** — feature flag possible (`config.yaml: duplicate_guard: enabled`). Rollback = revert + reactivate stub artifacts that got blocked. No data migration.

## Proposed Solution

Three layers, all in one PRD:

| Layer | Where | Effort |
|-------|-------|--------|
| L2 Guard | `cli/commands/new.rs` — search before create, prompt if similar found | ~50 LOC |
| L3 Health | `health/mod.rs` — duplicate detection check using existing search | ~80 LOC |
| L4 Stub rule | `validation/rules.rs` — new MUST rule, blocks activate | ~30 LOC |

Total: ~160 LOC, 0 new deps. Can be one PRD-043 in Sprint 13.7 (added to EPIC-003).

## Real cases of failure

| Artifact | Issue | Detected when |
|----------|-------|---------------|
| PRD-018 | Active stub (template body, R_eff=1.0 from no evidence) | 2026-04-07 audit |
| PRD-042 | Duplicate of PRD-018 created in same session | 2026-04-07 audit |

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| EPIC-003 | informs (Sprint 13 quality) |
| PRD-018 | based_on (real case of stub activation) |
| PRD-042 | based_on (real case of duplicate creation) |
| PRD-039 | informs (BM25 search needed for guard accuracy) |



