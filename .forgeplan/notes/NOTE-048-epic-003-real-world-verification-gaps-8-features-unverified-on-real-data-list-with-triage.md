---
depth: tactical
id: NOTE-048
kind: note
status: active
title: EPIC-003 real-world verification gaps — features unverified on real data, triage list
---

# NOTE-048: EPIC-003 real-world verification gaps

Post-v0.17.1 quality audit found 4 real bugs (PROB-030..033) plus 7
features with weak or no real-world verification. Unit tests pass
for all of them, but unit tests verify **correctness** (does code do
what we wrote), not **quality** (does the feature solve user's
problem better than before).

This NOTE is the planning input for a future verification sprint OR
for a v0.17.2 hotfix + separate test coverage pass.

## Real bugs found (4, shaped as PROBs)

| PROB | Title | Severity | Repro |
|---|---|---|---|
| **PROB-030** | BM25 "auth" prefix returns 0 results despite Authentication artifacts | HIGH (regression from substring) | confirmed |
| **PROB-031** | R_eff shows 1.00 when weakest evidence is CL0=0.1 (formula violation) | HIGH (corrupts FPF decisions) | confirmed |
| **PROB-032** | Search score display breakdown shows 0.0 components when total is non-zero | MEDIUM (UX lie) | confirmed |
| **PROB-033** | new evidence blocked by session state on fresh workspace | MEDIUM (blocks backfill workflow) | confirmed |

## Features verified but weakly (observed to work on happy path)

| Feature | What was tested | What was NOT tested |
|---|---|---|
| PRD-041 FPF rules | default 5 rules load + rendering | user-defined rules from config.yaml |
| PRD-041 FPF check | rule match on draft PRD | tie-breaking when multiple rules at same priority |
| PRD-040 R_eff CI | "insufficient (1 evidence)" label | CI bounds math with 3+ evidence |
| PRD-043 Duplicate guard | detects similar titles during new | similarity threshold tuning, false-positive rate |
| PRD-043 Stub detection | fires on fresh template | custom content with placeholders |
| PRD-044 Reindex trim | orphan relation cleanup on dogfood | corrupt-kind path (no test workspace had one) |
| PRD-045 Health verdict | concrete actions on dogfood | gradient levels (only binary tested) |

## Features NOT verified on real data (7+)

### Sprint 13.2 PRD-039 Smart Search v2

- **Composable ArtifactFilter DSL** — `--status active --depth deep
  --with-evidence --since 2026-04-01` combined queries on realistic
  workspace never tested beyond single-flag cases
- **Graph expansion 1-hop** — neighbor inclusion never verified
  affects ranking appropriately
- **BM25 ranking quality vs old substring** — no A/B benchmark on a
  set of real queries with known relevance judgments

### Sprint 13.3 PRD-035 p1 Tags + SourceTier

- **Tag canonicalization** — `source=code` vs `Source=Code` handling
- **SourceTier T1→CL3 mapping** — precedence with explicit CL fields
  never tested with conflict cases
- **Multiple tags per artifact** — `tag X a=1 b=2 c=3` flow

### Sprint 13.4 PRD-035 p2 Discover MCP

- **Full discover session E2E** — `discover` → multiple `finding`
  calls → `complete` never run on a real brownfield project
- **Session state persistence** — what happens when session
  crashes mid-way
- **Phase transitions during discovery** — documented but not tested

### Sprint 13.5 PRD-040 Scoring Intelligence

- **Skills Memory adaptive routing** — requires accumulated routing
  history, never tested over 10+ routing decisions
- **Exponential decay (90-day half-life)** — decay curve behavior
  never verified beyond unit tests
- **Confidence threshold 0.6** — picked in code, no real data to
  validate it's the right value

### Sprint 13.6 PRD-041 FPF Rules

- **User-defined rules via .forgeplan/config.yaml** — config loading
  path works (unit-tested) but never exercised with complex user rules
- **Rule priority ties** — two rules at same priority, which wins?

### Sprint 13.7 PRD-042 FPF KB Vector Search

- **BGE-M3 semantic precision vs keyword on real queries** — the
  `#[ignore]` test runs one trivial query ("confidence in evidence"),
  never measured precision/recall on 200+ FPF sections
- **--semantic fallback warning when feature ON but model fails**
  — multiple runtime fallback paths unit-tested via closure injection
  but not with real Embedder failing

### Cross-feature workflows

- **`forgeplan estimate` command** — listed in CLI help, never ran
  on a real PRD to verify estimation quality
- **Real v3→v4 schema migration on live workspace** — unit test
  fixture exists, no end-to-end migration of a user's actual
  workspace
- **Reindex on 500+ artifact workspace** — performance untested
- **MCP tools via real Claude Code client** — handler unit tests
  exist, never exercised via actual MCP protocol invocation from
  a client

## Triage recommendation

### Track 1: v0.17.2 hotfix (2 HIGH bugs, ~2 hours each)

- **PROB-030** BM25 prefix fallback (add substring on 0 BM25 results)
- **PROB-031** R_eff weakest-link formula investigation + fix
- Shape PRD-046, PRD-047 per /forge
- ADI with code investigation (lesson from v0.17.1)
- Full cycle: code → tests → audit → evidence → activate → PR → tag

### Track 2: Polish hotfix bundled with Track 1

- **PROB-032** search score breakdown display (fix or remove)
- **PROB-033** new evidence session state loosening
- Lower severity but quick wins

### Track 3: Verification sprint (Sprint 14.x)

Dedicated test coverage + real-world verification pass. Focus on the
7 feature areas above. Produces:

- 10+ integration tests covering real workflows
- Benchmark: BM25 vs substring precision/recall on labelled query set
- Benchmark: BGE-M3 semantic search on 200+ FPF sections
- Skills Memory training test with 20+ routing decisions
- Real v3 workspace migration integration test
- MCP real client test via mock Claude Code harness
- Performance stress test on 500+ artifact workspace

Produces one or more EVIDs documenting real quality measurements,
not just unit test counts.

## Lessons for NOTE-044 (methodology)

Add to Phase 5 Manual UX verification:

- **Dogfood test does NOT equal real-world test.** Dogfood is a
  specific workspace with specific shape. Real-world verification
  needs variety: fresh workspace, medium workspace, large workspace,
  workspace with accumulated history.
- **Quality != correctness.** Tests that assert "function returns
  non-empty" don't prove the function solves the user's problem.
  Add a "quality criteria" checklist per feature: before marking
  done, answer "what would the user try first, and does our feature
  handle it gracefully?"
- **Prefix search is a common user expectation** — grep-like
  behavior is the default mental model. Any new search system must
  handle prefix queries or explicitly document they are not
  supported.
- **Root cause investigation is not bug triage.** Found bugs must
  be investigated deeply (not just reported) before fix planning.
  PROB-031 R_eff bug needs code walkthrough to find the actual
  disconnect, not just "there's a number mismatch".
- **Formula correctness is verifiable via exhaustive enumeration.**
  For small state spaces (like CL × verdict × evidence_type × stale),
  write a test that enumerates every combination and asserts the
  formula produces the expected output. Unit tests that check
  individual cases miss boundary interactions.

## Related

| Artifact | Relation |
|---|---|
| PROB-030 | informs (real bug found in audit) |
| PROB-031 | informs (real bug found in audit) |
| PROB-032 | informs (real bug found in audit) |
| PROB-033 | informs (real bug found in audit) |
| NOTE-044 | refines (adds lessons to sprint checklist) |
| EPIC-003 | context (verification debt from the entire epic) |
| EVID-065 | precedent (backfill pattern demonstrating PROB-033 workaround) |
