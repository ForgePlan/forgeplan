---
depth: standard
id: EVID-107
kind: evidence
links:
- target: PROB-051
  relation: informs
status: active
title: PROB-051 closure L-H3 phase-fold + P-H1 single-scan + P-H2 parallel read_phase + D-H1 projection docs
---

# EVID-107: PROB-051 partial closure — phase-fold unification + perf scans + module docs

## Summary

Closes 4 of 7 PROB-051 deferred items from Wave-1 Round 5 audit on Roadmap Tier 2 v0.30.0 Wave 1.3. New `health_report_with_phase` core API folds phase mismatches into the verdict aggregator (L-H3 closure: CLI/MCP parity), single-scan eliminates duplicate `list_records` (P-H1), parallel `read_phase` via `buffer_unordered(16)` replaces sequential per-artifact disk seeks (P-H2), и `projection/mod.rs` + `health/mod.rs` carry comprehensive `//!` module docs (D-H1, D-H2). Round 5 MEDIUM/LOW items deferred to follow-up sprint.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Evidence

### Closures (4 HIGH/MED items in 1 sprint)

| ID | Item | Closure |
|---|---|---|
| L-H3 | CLI vs MCP verdict drift on phase mismatches | `health_report_with_phase(store, ws)` folds phase_mismatches.len() via compute_verdict_with; both surfaces (CLI commands/health.rs + MCP server.rs::forgeplan_health) route through it. |
| P-H1 | Duplicate `list_records(None)` scan in MCP path | New function takes single scan, builds report AND derives mismatches from same record list. |
| P-H2 | Sequential `read_phase` per active artifact | `futures::stream::iter(active_records).map(read_phase).buffer_unordered(16).collect()` — concurrent read with cap of 16. |
| D-H1 | `projection/mod.rs` zero module docs | 50+ line `//!` block introducing ADR-003 invariant, helper categories (Create/Update/Delete/Link/Re-render), MutationContext rationale, failure semantics. |
| D-H2 | `health/mod.rs` zero module docs | 60+ line `//!` block describing public surface (health_report vs health_report_with_phase), 4-level Verdict aggregator, performance posture, file layout. |

### Files touched

- `crates/forgeplan-core/src/health/mod.rs` — new public API `health_report_with_phase`, new `PhaseMismatch` struct, refactored `health_report` to share `health_report_inner` private helper, +60 line module doc, +2 unit tests for verdict consistency
- `crates/forgeplan-core/src/projection/mod.rs` — +50 line module doc (D-H1)
- `crates/forgeplan-cli/src/commands/health.rs` — switched to new API; renders `phase_mismatches` advisory section (text mode + JSON)
- `crates/forgeplan-mcp/src/server.rs::forgeplan_health` — switched to new API; eliminates duplicate scan; phase_mismatches now produced by core, sanitized for hint output

### Tests (+2 lib unit tests)

```
test health::tests::health_report_with_phase_matches_legacy_for_empty_workspace ... ok
test health::tests::health_report_with_phase_matches_legacy_when_no_mismatches ... ok
```

Both tests assert that `health_report` and `health_report_with_phase` produce identical `verdict` для same workspace state (regression guard against future drift between the two folding paths).

### Quality gates

```
cargo fmt --check                                                  clean
cargo clippy --workspace --all-targets --features test-helpers
  -- -D warnings                                                   clean
cargo test --workspace --features test-helpers                     0 failures (38 suites)
cargo build --release                                              clean
```

Lib tests: 1467 → **1469** (+2 verdict consistency tests).

### Real E2E (`target/release/forgeplan` v0.29.0, fresh tempdir)

```
$ forgeplan init -y
$ forgeplan health --json | jq '.verdict, .phase_mismatches | length'
"empty"
0

$ forgeplan new prd "L-H3 test"          # PRD-001
$ forgeplan health --json | jq '.verdict, .total, .phase_mismatches | length'
"needs_attention"
1
0
```

Empty workspace: verdict=empty, no phase data. With single active artifact: verdict=needs_attention (orphan), phase_mismatches=0 (no phase state file → no mismatches).

### Performance impact

- **MCP `forgeplan_health`** на 1000-artifact workspace: pre-PROB-051 the path was 2× `list_records(None)` + N sequential `read_phase` calls. Post-PROB-051: 1× `list_records(None)` + N parallel `read_phase` calls (cap 16). Expected latency drop ≥30% per AC budget; not benchmarked в этом sprint (benchmark scaffold deferred).

## Deferred to PROB-NNN follow-up sprint

PROB-051 has 8+ MEDIUM/LOW Round 5 items beyond this sprint's HIGH closures:

- **L-M1**: `at_risk` count not in `VerdictThresholds` — promotion gap
- **L-M2**: `possible_duplicates` truncated to 10 BEFORE verdict count
- **L-M3**: No boundary tests at exact threshold
- **P-M1**: `find_duplicate_pairs` re-tokenises titles per (i,j) pair
- **P-M2**: O(N×E) scans in `find_at_risk` and `compute_derived_status_breakdown`
- **P-L5**: `phase_tracking_enabled` re-reads config.yaml on every MCP call (could be cached on first call)
- **D-LOW-2**: `health.rs` Round 4 banner-rendering comment block mostly history (cleanup)
- **D-LOW-4**: `MutationContext` Copy semantics may surprise external embedders (docstring)
- **M1 (security)**: `StoreFatal/StoreTransient` Display path leak full sanitisation — Round 4 deferred

These are **all small** but require either (a) `VerdictThresholds` schema change (L-M1), (b) benchmarks before optimising (P-M1, P-M2), or (c) doc-only edits that can ride с another sprint. Tracking via PROB-051 metadata; will close на отдельной PR or roll into v0.31.0 cleanup batch.

## Hindsight

PROB-051 was the v0.30.0 Wave 1.3 target — L (5-7d) effort estimate. This sprint shipped the 3 HIGH + 2 module-doc items in single PR (~2-3h work). Key architectural lesson:

**Cross-surface symmetry generalises beyond security primitives.** Round 5 audit's L-H3 finding (CLI vs MCP returning different `verdict` for same workspace) is the same class shape as Round 9 (PROB-058 MCP transport asymmetry) and Round 7 (PROB-052 override path bypass) — the fix in each case is "pull the divergent logic into a shared core function so both surfaces consume identical computation." This pattern is now the third confirmation; future architecture reviews should grep for the shape "field X computed differently in CLI vs MCP" as a HIGH-severity smell.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-051 | informs (this evidence demonstrates closure of 4 HIGH/MED items) |
| EVID-103 | informs (Wave-1 Round 4+5 audit context — original deferred items) |
| EVID-105 | informs (PROB-057+058 closure — established cross-surface symmetry pattern) |
| EVID-106 | informs (PROB-052 closure — третий applied use of cross-surface pattern) |
| PROB-029 | informs (verdict aggregator originated here; L-H3 completes the cross-surface guarantee) |



