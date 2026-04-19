---
depth: tactical
id: EVID-077
kind: evidence
links:
- target: PRD-057
  relation: supports
status: draft
title: PRD-057 multi-agent dispatcher — R3 audit closed, 1391 tests, full pipeline E2E
---

# EVID-077: PRD-057 multi-agent dispatcher — R3 audit closed, 1391 tests, full pipeline E2E

| Field | Value |
|-------|-------|
| Status | Draft |
| Created | 2026-04-19 |
| Valid Until | 2026-07-19 |
| Target | PRD-057 |

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

End-to-end validation of the PRD-057 multi-agent dispatcher stack (Inc 2
agent identity + Inc 3 claim protocol + Inc 4 orchestrator dispatcher),
including the R2 and R3 adversarial audit hotfixes. All seven feature
commits on `feat/prd-057-inc-2-4-dispatcher` from 0331c38 through
be13960 verified together.

Validation methods:
1. `cargo test --workspace` — full unit + integration suite on the
   commit stack (7 commits: Inc 2, Inc 3, R2 hotfix, Inc 4, R3 hotfix,
   dogfood E2E, workflow variations).
2. `cargo fmt --all --check` + `cargo clippy --workspace --all-targets
   -- -D warnings` — zero diffs, zero warnings on the strict CI gate.
3. Multi-agent adversarial audits: 3-agent mid-sprint panel after Inc 3
   and 4-agent final panel after Inc 4 (rust-pro + security-expert +
   architect-reviewer + production-validator). Every HIGH/MED finding
   closed via targeted commits with regression tests.
4. PRD-057 per-FR / per-AC task-completion audit: every FR-001..014
   mapped to specific implementation location + test, every AC-1..5
   mapped to covering test.

## Result

**Tests**: 1391 passing / 0 failing across 16 test binaries. Delta vs
pre-PRD-057 baseline: +94 tests (identity + claim + dispatch cores +
MCP wiring + dogfood + workflow + AC-4 concurrent E2E + audit
regression guards).

**FR coverage (14/14)**:
- FR-001..003 → `forgeplan_core::dispatch::compute_dispatch_plan` +
  graph::topological::kahn_sort integration in MCP handler.
- FR-004..006, FR-014 → `forgeplan_core::claim::ClaimStore` +
  `forgeplan_claim/release/claims` MCP tools; `.forgeplan/claims/` in
  .gitignore.
- FR-007, FR-008 → `workspace::lock` guards held across every write
  handler (forgeplan_new, _update, _delete, _claim, _release,
  _phase_advance, _supersede, _deprecate); dispatcher and claims list
  are lockless reads per R2 audit architect MED.
- FR-009 → `ForgeplanServer::current_identity` captured from
  `peer.peer_info()`, stamped into frontmatter via
  `projection::stamp_agent_identity` on new + update paths.
- FR-010..011 → `normalize_dispatch_domain` + `skill_match` in core;
  per-agent `agent_skills` via `DispatchParams`.
- FR-012 → `forgeplan_health` response body carries `active_claims` +
  `active_claim_count` + `skipped_claim_files`.
- FR-013 → `forgeplan_get` `_next_action` hint appends claim holder
  and expires_at when a live claim exists.

**AC coverage (5/5)**:
- AC-1 → `disjoint_artifacts_go_to_separate_buckets` +
  `overlapping_artifacts_force_one_to_serial` + dogfood variant.
- AC-2 → `claim_rejects_active_different_agent` +
  `claim_rejects_existing_claim_by_different_agent` (MCP boundary).
- AC-3 → `claim_takes_over_expired_claim` (Core).
- AC-4 → `concurrent_forgeplan_new_emits_unique_ids` spawns 3 parallel
  MCP calls on one workspace and asserts 3 distinct IDs + 3 markdown
  files. Previously covered only by the Inc 1 lock-level proxy test.
- AC-5 → `stamp_best_effort_writes_captured_identity` +
  `render_projection_preserves_unknown_fm_across_rerender` +
  `dispatch_workflow_full_cycle_new_claim_update_release_redispatch`.

**Audit findings closed**:
- R2 mid-sprint (3 agents): 3 HIGH + 5 MED + 1 LOW — path traversal in
  ClaimStore, release empty-agent bypass, fragile forgeplan_release
  force path, atomic tempfile+rename writes, Unicode/control char
  rejection in AgentIdentity, MAX_CLAIM_FILE_BYTES cap, lockless
  forgeplan_claims (read-only). Regression guards: 14 tests.
- R3 final (4 agents): 2 HIGH + 7 MED + 6 LOW + 1 FR gap + 2 FR missing
  — unbounded agents OOM → MAX_AGENTS clamp, affected_files fragmented
  (FM key vs markdown section) → extract_affected_files body fallback,
  blocked artifacts filtered out of dispatch, FR-012 + FR-013 added,
  Jaccard boundary `>=`, scalar form accept, Unicode domain rejection,
  MAX_SKILLS_PER_AGENT + MAX_AFFECTED_FILES bounds, defensive id
  traversal guard, claimed_set case-normalization. Regression guards:
  11 tests.

**Dogfood E2E (10 tests)**: empty workspace, 1-agent conflict serialize,
5-agent distribute evenly (PRD-057 target upper bound), over-MAX
rejection, markdown-section fallback, blocked-artifact skip, claim/
release full cycle, skill routing, health claims surface, forgeplan_get
claim hint. All exercise the real ForgeplanServer + LanceStore +
filesystem projection stack.

**Workflow coverage (4 tests)**: kind filter, threshold=0 conservative,
threshold=1 permissive, full new→claim→identity→release→dispatch cycle.

## Interpretation

PRD-057 delivers the core value proposition end-to-end: a single
`forgeplan_dispatch --agents N` call hands the orchestrator a
parallel-safe work plan for N sub-agents with explicit reasoning.
Inc 2 identity tracking closes the "who did what" audit gap. Inc 3
claim protocol gives sub-agents the soft-coordination signal the PRD
called for, with TTL safety against silent crashes (NFR-004). The
write-serialization guarantee (FR-007, FR-008) is carried forward from
Inc 1 and extended to every new write handler introduced in Inc 3 via
the audit-validated `workspace_lock` pattern.

Two-round adversarial audit pressure applied AFTER feature commits
found 21 substantive issues (5 HIGH, 12 MED, 7 LOW, 3 missing FR / gap)
that unit tests missed — consistent with the established pattern across
PRD-055, PRD-056, PRD-057. Every one is now closed with a targeted
regression test, raising the test count to 1391.

The stack is ready to ship as v0.24.0. Deferred items (shared kv_yaml
abstraction, HTTP/SSE identity, DispatchDecision enum for i18n, agent
profiles persistence) are architecture improvements documented in PRD
progress and do not block the current functional scope.

## Congruence Level Justification

CL3 (same-context): the evidence is the project's own test suite and
audit trail against the exact artifact under review (PRD-057). The
tests run against the production code paths, not mocked/simulated
equivalents. Measurement artifacts (test counts, audit commits, code
locations) are all traceable in-repo.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-057 | supports |
| EPIC-005 | informs |

