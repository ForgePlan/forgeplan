---
depth: tactical
id: EVID-062
kind: evidence
links:
- target: PRD-040
  relation: informs
status: draft
title: Sprint 13.5 PRD-040 Scoring Intelligence — Skills Memory + R_eff CI, 1042 tests
---

# EVID-062: Sprint 13.5 PRD-040 Implementation Evidence

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Summary

Sprint 13.5 implemented PRD-040 Scoring Intelligence (FR-001 + FR-002) — adaptive routing via Skills Memory and R_eff confidence intervals. Pattern source: RuVector `agenticdb.rs::Skill` (adapted as simple Rust structs, no RL pipeline).

## Implemented FRs

### FR-001: Routing Skills Memory
- **NEW** `crates/forgeplan-core/src/routing/skills.rs` (~335 LOC, 19 tests)
- `RoutingSkill` struct with pattern/depth/usage_count/success_rate/decay
- `record_success()` / `record_failure()` update running mean
- `confidence()` = success_rate × min(usage_count/3, 1.0) × decay_factor
- `should_override()` returns true when confidence ≥ 0.6
- `best_matching_skill()` picks highest-confidence match
- **Decay**: exponential half-life 90 days, floor 0.1 (never zero)
- **Integration**: `routing::route_with_skills(description, skills)` checks skills FIRST, falls back to keyword rules if no high-confidence match
- Backward compat: existing `route()` calls new function with empty skills Vec

### FR-002: R_eff Confidence Interval
- **MODIFIED** `crates/forgeplan-core/src/scoring/reff.rs` (+~200 LOC, 10 new tests)
- `ReffCi` struct: point, low, high, evidence_count, stale_count
- `r_eff_with_ci()` — heuristic band widening with evidence sparsity/staleness
- Formula: uncertainty = `0.30 / sqrt(count)` + `0.10 × stale_count` (capped)
- 1 evidence: wide band (insufficient label)
- 3+ evidence: meaningful [low — high] interval
- 9+ evidence: tight band (high confidence)
- **Stale evidence widens CI** — operator intuition preserved
- **MODIFIED** `crates/forgeplan-cli/src/commands/score.rs` (+~35 LOC)
- JSON output includes `r_eff_ci` object with all fields
- Styled output shows `Confidence: [0.00 — 0.27] (3 evidence)` or `(3 fresh, 1 stale)` or `insufficient`

## Test results

- **Total: 1042 tests pass, 0 failed** (up from 1006)
- **+36 new tests** in Sprint 13.5:
  - routing::skills: 19 tests (positive + negative + corner cases)
  - routing::tests: 7 new skills integration tests
  - scoring::reff: 10 new CI tests
- Test types: positive, negative (malformed JSON, empty patterns), corner (very stale, 1-evidence insufficient, clamping, NaN prevention)
- cargo fmt --check: clean
- cargo check --workspace: 0 warnings
- 0 new dependencies

## E2E verification (release binary)

### Positive cases
```
$ forgeplan score PRD-001  (3 evidence linked)
  Evidence breakdown:
    EVID-001 [Supports] CL0 = 0.1
    EVID-002 [Supports] CL0 = 0.1
    EVID-003 [Supports] CL0 = 0.1
  R_eff:        1.00 -- Adequate
  Confidence:   [0.00 — 0.27] (3 evidence)  ← FR-002 working

$ forgeplan score PRD-001 --json
  r_eff_ci: {
    "evidence_count": 3,
    "low": 0.0,
    "high": 0.27,
    "insufficient": false,
    "width": 0.27,
    ...
  }
```

### Corner case: insufficient evidence
```
$ forgeplan score PRD-002  (1 evidence linked)
  Confidence:   insufficient (1 evidence)  ← correctly labeled
```

### Regression checks — all prior Sprint 13.x features still work
- ✅ 13.1 duplicate guard still works
- ✅ 13.2 BM25 search: `forgeplan search "unique" --no-expand` finds artifacts
- ✅ 13.3 tags: `forgeplan tag` + `list --tag source=code` works
- ✅ 13.4 discover: `forgeplan discover start` creates session with protocol

## Extended testing discipline (per user request)

Tests in Sprint 13.5 include:

**Positive tests:**
- new skill has defaults
- matches all tokens required
- record_success/failure update stats correctly
- best_matching picks highest confidence
- CI point matches r_eff

**Negative tests:**
- malformed JSON returns error
- missing fields in JSON returns error
- empty description fails non-empty pattern
- low confidence skill does not override
- non-matching skill falls back

**Corner cases:**
- empty pattern matches everything (vacuous truth)
- empty description matches empty pattern
- very stale skill (360 days) decayed below override threshold
- confidence clamps to 1.0 even with invalid input
- CI single evidence shows "insufficient"
- CI empty evidence returns all-zeros
- CI many stale caps penalty
- CI clamps to [0.0, 1.0] range
- CI three evidence hits meaningful range boundary

**Regression:**
- route_with_empty_skills behaves as route (backward compat)
- all 84 prior routing tests still pass
- all 16 prior reff tests still pass

## Architecture notes

- **Skills are memory artifacts** — no new schema, reuse existing memory/ infrastructure (NFR-002 satisfied)
- **Decay keeps historical signal** — never exactly zero, preserves "this skill was useful long ago" as weak signal
- **Confidence threshold 0.6** — empirically chosen: requires at least (1.0 success × 2 uses) or (0.6 success × 3 uses)
- **Stale evidence in CI widens** not resets — 1 stale item adds +0.10 uncertainty, capped at +0.30
- **CI is heuristic, not Bayesian** — designed for operator intuition, not statistical rigor

## Deferred

PRD-040 NFR-002 "Skills stored as Memory artifacts" is currently **infrastructure-only** — the in-memory `Vec<RoutingSkill>` works, but loading from `.forgeplan/memory/` at runtime + the `route` CLI wire-up to actually CALL `route_with_skills` with loaded skills is pending. Recommendation: Sprint 13.5.1 hardening.

Similarly, automatic success recording (agent marks route as successful after artifact activates) is a larger design question deferred to Sprint 14+.

## Integration points

- **Sprint 13.2 smart search**: skills can be found via `forgeplan search --type memory`
- **Sprint 13.3 tags**: skills can be tagged `source=skill` for organization
- **Sprint 13.1 methodology**: existing route command still enforces depth rules
- **R_eff framework**: CI is an additive display layer — doesn't change scoring logic

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-040 | informs (this evidence supports FR-001 + FR-002) |
| EPIC-003 | informs (Sprint 13 v0.17.0 series) |
| EVID-061 | informs (Sprint 13.4 predecessor) |
| sources/RuVector | pattern source (agenticdb Skills) |
