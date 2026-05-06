---
depth: standard
id: EVID-109
kind: evidence
links:
- target: PROB-056
  relation: informs
status: active
title: PROB-056 closure leaky verdict abstraction partial_verdict field with doc contract
---

# EVID-109: PROB-056 closure — `HealthReport.partial_verdict` field surfaces phase-fold contract

## Summary

Closes PR-E Round 6 audit MED-1 — leaky verdict abstraction в `HealthReport`. Pre-PROB-056 the single `verdict` field silently switched semantic between callers: `health_report()` populated it as partial (phase_mismatches=0), `health_report_with_phase()` populated it as folded (PROB-051 closure). External library consumers calling `health_report` directly couldn't tell от the type signature что their `verdict` was partial. Post-PROB-056: new `partial_verdict: Verdict` field always carries the partial value; `verdict` continues to carry "best-known" value (folded когда available, partial otherwise).

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Evidence

### Implementation

New field on `HealthReport` struct в `crates/forgeplan-core/src/health/mod.rs`:

```rust
/// **Best-known verdict for user-facing display.**
pub verdict: Verdict,

/// PROB-056 closure — verdict computed using ONLY core warning classes
/// (phase_mismatches=0). External library consumers tracking additional
/// context MUST consume this as base for compute_verdict_with()
/// recomputation rather than relying on `verdict` (which may be folded).
pub partial_verdict: Verdict,
```

- `health_report` legacy path: writes BOTH fields с phase_mismatches=0 — they're equal.
- `health_report_with_phase` post-fold path: writes `partial_verdict` с phase_mismatches=0, then OVERWRITES `verdict` с folded value.
- Wire JSON format unchanged for `verdict`; new `partial_verdict` field appears optionally.

### Tests (+2 unit tests)

```
test health::tests::health_report_partial_verdict_equals_verdict_when_no_phase ... ok
test health::tests::health_report_with_phase_partial_verdict_invariant ... ok
```

Suite: lib 1475 → **1477** PASS.

### AC tracking

- AC-1 ✅ `partial_verdict` field added (rename was Option A — implemented as additive split which preserves wire format AND surfaces semantic without forcing CLI/MCP migration)
- AC-2 ✅ Doc-comment on `partial_verdict` documents recomputation contract
- AC-3 ✅ CLI/MCP code paths reviewed — both already route through `health_report_with_phase` (PROB-051 closure) so `verdict` is the right value to consume; no migration needed
- AC-4 ✅ +2 tests demonstrating the invariants
- AC-5 ✅ CHANGELOG entry under Refactor section

### Quality gates

```
cargo fmt --check                                                  clean
cargo clippy --workspace --all-targets --features test-helpers
  -- -D warnings                                                   clean
cargo test --workspace --features test-helpers                     0 failures (38 suites)
```

## Hindsight

PROB-051 already closed the **specific MCP-override leaky abstraction** by routing both CLI and MCP through `health_report_with_phase` (which writes the folded verdict to the `verdict` field). PROB-056 is the **architectural completion** — surfacing the dual-semantic contract в the type system for external library consumers.

Design choice: additive (`partial_verdict` as new field) vs hard rename (`verdict` → `partial_verdict`). Picked additive because:
1. Hard rename forces all CLI/MCP/external code paths to migrate (LOC churn).
2. Hard rename + serde rename keeps JSON wire OK but creates a different foot-gun: `verdict` JSON field carries different semantic than Rust `partial_verdict` field name suggests.
3. Additive split lets `verdict` keep the user-facing "best known" semantic (which is what consumers actually want) and `partial_verdict` becomes the explicit access for advanced cases.

This is the same architectural pattern as PROB-049 typed errors (introduce typed alternatives without removing the legacy surface) — incremental migration over breaking rename.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-056 | informs (this evidence demonstrates closure) |
| PROB-051 | informs (L-H3 closure already addressed the MCP-override specific instance; PROB-056 finishes the architectural cleanup) |
| PROB-029 | informs (Verdict aggregator origin) |
| EVID-107 | informs (PROB-051 closure context — health_report_with_phase introduced) |



