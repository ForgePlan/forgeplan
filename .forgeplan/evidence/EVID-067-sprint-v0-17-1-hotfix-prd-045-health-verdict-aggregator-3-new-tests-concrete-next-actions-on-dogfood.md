---
depth: tactical
id: EVID-067
kind: evidence
links:
- target: PRD-045
  relation: informs
- target: PROB-029
  relation: informs
status: active
title: Sprint v0.17.1 hotfix PRD-045 health verdict aggregator — 3 new tests, concrete next actions on dogfood
---

# EVID-067: PRD-045 Implementation Evidence (v0.17.1)

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Summary

Sprint v0.17.1 hotfix PRD-045 shipped. `generate_next_actions` in
`health/mod.rs` extended to read `possible_duplicates` and
`active_stubs` signals. Compute order reshuffled so signals are
available before the aggregator runs. Verdict no longer says
"Project looks healthy" when stubs or duplicates are present.

## Commit

`b6f478e` on `release/v0.17.1` branch (shared with PRD-044 as
paired hotfix commit).

## FR mapping

### FR-001: Aggregator reads stubs + duplicates

File: `crates/forgeplan-core/src/health/mod.rs` lines 119-138.
Before:
```rust
let next_actions = generate_next_actions(total, &by_status,
    &blind_spots, stale_count, &orphans);
let possible_duplicates = find_duplicate_pairs(...);  // AFTER
let active_stubs = find_active_stubs(...);             // AFTER
```

After:
```rust
let possible_duplicates = find_duplicate_pairs(...);   // BEFORE
let active_stubs = find_active_stubs(...);              // BEFORE
let next_actions = generate_next_actions(total, &by_status,
    &blind_spots, stale_count, &orphans,
    &possible_duplicates, &active_stubs);
```

Signature extended with 2 parameters: `possible_duplicates:
&[DuplicatePair]` and `active_stubs: &[ActiveStub]`.

### FR-002: Three-level verdict — DEFERRED to v0.18

Explicit Verdict enum not shipped in v0.17.1. The existing "next_actions
is non-empty when warnings exist" signal is sufficient for v0.17.1
goal (don't say "looks healthy" when it isn't). Making verdict a
typed enum requires HealthReport shape change which would bump the
MCP JSON contract — breaking for existing tools. Deferred to v0.18
minor release where shape changes are acceptable.

### FR-003: Next actions with concrete IDs

Lines 314-336: when stubs present, action is
`"Fill or supersede N active stub(s) — e.g. 'forgeplan supersede
PRD-008 --by <NEW>' or 'forgeplan deprecate PRD-008 --reason
\"abandoned\"'"` using the first stub's ID as the concrete example.
Same pattern for duplicates with first pair's IDs.

### FR-004: CHANGELOG entry

`CHANGELOG.md` v0.17.1 Fixed section includes this bug with problem
statement and fix description.

## Tests

3 new unit tests added in `health/mod.rs`:

1. `next_actions_includes_stub_remediation_when_stubs_present`
   — asserts stub ID appears in output and "looks healthy" does NOT
2. `next_actions_includes_duplicate_remediation_when_dups_present`
   — asserts duplicate pair IDs appear and "looks healthy" does NOT
3. `next_actions_says_healthy_when_no_warnings`
   — regression test: empty warnings should still show "healthy"

Existing `next_actions_capped_at_three` test updated to pass empty
vecs for the new parameters (backward-compat verification).

Total tests: 1128 → 1131 (+3).

## Dogfood verification

Before fix:
```
⧗ Possible duplicates (5):
  EVID-001 ↔ EVID-003 (100%) — "Dogfood lifecycle test"
  ...

⚠ Active stubs (8):
  PRD-008 (prd) "CLI UX Redesign" — 6 markers
  ...

→ Next actions:
  1. Project looks healthy. Continue implementation.

Project looks healthy!
```

After fix:
```
⧗ Possible duplicates (5): ...
⚠ Active stubs (8): ...

→ Next actions:
  1. Fill or supersede 8 active stub(s) — e.g. `forgeplan supersede PRD-008 --by <NEW>` or `forgeplan deprecate PRD-008 --reason "abandoned"`
  2. Resolve 5 duplicate pair(s) — e.g. `forgeplan deprecate EVID-003 --reason "duplicate of EVID-001"`
  3. Create evidence for 2 artifact(s) without proof
```

"Project looks healthy" message eliminated. User gets concrete
remediation commands they can copy-paste.

## Quality gates

- cargo fmt --check: clean
- cargo check --workspace: 0 warnings
- cargo clippy --workspace --all-targets -- -D warnings: clean
- cargo test --workspace: 1131 pass, 0 fail
- Manual dogfood verification: verdict no longer contradicts warnings

## Related

| Artifact | Relation |
|---|---|
| PRD-045 | informs |
| PROB-029 | informs (closed) |
| PRD-043 | informs (Sprint 13.1 detection whose signals this aggregates) |
| EVID-058 | informs (Sprint 13.1 original implementation) |
| EVID-066 | sibling (PRD-044 paired work) |

