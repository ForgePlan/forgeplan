---
depth: standard
id: PROB-056
kind: problem
last_modified_at: 2026-05-05T20:39:32.356595+00:00
last_modified_by: claude-code/2.1.128
links:
- target: PROB-029
  relation: based_on
status: draft
title: PR-E Round 6 deferred — compute_verdict_with leaky abstraction
---

## Signal

PR-E Round 6 adversarial architectural audit (3 parallel agents, 2026-05-05)
flagged `HealthReport.verdict` (stored field) as a leaky abstraction:

```rust
// crates/forgeplan-core/src/health/mod.rs:149-162
pub fn compute_verdict(&self) -> Verdict {
    compute_verdict(self, &VerdictThresholds::default(), 0)
}

pub fn compute_verdict_with(
    &self,
    thresholds: &VerdictThresholds,
    phase_mismatches: usize,
) -> Verdict { ... }
```

`HealthReport.verdict` (line 139, ~`pub verdict: Verdict`) is populated
during `health_report()` with `phase_mismatches=0`, because the `core`
crate doesn't know about MCP-side `advisory_phase_mismatches`. MCP server
then re-computes:

```rust
// crates/forgeplan-mcp/src/server.rs:~2719
let verdict = report.compute_verdict_with(
    &VerdictThresholds::default(),
    phase_mismatches.len(),
);
```

…silently overriding `report.verdict`. CLI consumers who naively read
`report.verdict` (e.g. `commands/health.rs:74`) get a verdict that
disagrees with MCP for the same workspace.

The "MCP knows about phase mismatches, core doesn't" gap is a real
bounded-context split, but **storing a half-computed `verdict` in the
struct invites stale reads**. Audit reasoning: the field acts as a
foot-gun for any future consumer that doesn't realize the recomputation
is mandatory.

By-design today (Round 5 deferred); audit elevated to MED because
v0.29.0 added `Verdict::Empty` (PR-E Round 6) which makes the
inconsistency more visible.

## Constraints

- MUST NOT break the existing `--json` output schema (consumers may
  already gate on `verdict` field — removal is breaking).
- MUST NOT force every consumer to know about `phase_mismatches`
  (CLI can legitimately ignore it — the bounded-context split is
  intentional).
- Solution should be **discoverable** at the type level (a comment is
  not enough — audit found this exact comment-only mitigation
  insufficient).

## Optimization Targets (1-3 max)

- **Option A (preferred)**: rename stored field to `partial_verdict`,
  add doc-comment that recomputation with caller-side context
  (phase_mismatches) is mandatory before user-facing display. Keep
  serde wire field as `verdict` for backwards compatibility (rename
  via `#[serde(rename = "verdict")]`).
- **Option B**: remove stored field entirely, force every consumer
  to call `compute_verdict_with(...)`. Breaking change for `--json`
  consumers (would need `verdict_summary` populated by callers too).
- **Option C**: add a runtime assertion in `compute_verdict_with` that
  the recomputed value matches stored value when `phase_mismatches ==
  0`, panicking on mismatch. Keeps the type, surfaces drift loudly.

Recommend **Option A** — minimum-blast-radius fix that surfaces the
contract at the type level without breaking JSON consumers.

## Observation Indicators (Anti-Goodhart)

- Test count must stay ≥ baseline.
- `forgeplan health --json` output schema unchanged for consumers
  (verdict field still present at JSON top level).
- `partial_verdict` only exists at the Rust type level; never appears
  in JSON output or CLI text rendering.

## Acceptance Criteria

- [ ] `HealthReport.verdict` renamed to `partial_verdict`.
- [ ] `#[serde(rename = "verdict")]` annotation preserves JSON wire
  format.
- [ ] Doc comment on `partial_verdict` documents the recomputation
  contract: "Stored value computed with `phase_mismatches=0`. Callers
  with phase-mismatch context MUST call `compute_verdict_with(...)`
  before user-facing display."
- [ ] CLI + MCP code paths reviewed: every reader of `partial_verdict`
  is either (a) explicitly opting out of recomputation with comment,
  or (b) recomputing.
- [ ] +1 doc-test demonstrating the recomputation pattern.
- [ ] CHANGELOG entry under **Refactor** section + migration note for
  downstream library consumers.

## Refs

- PR-E Round 6 audit (2026-05-05): architect-reviewer agent MED-1
- CHANGELOG.md (v0.29.0): "Deferred to v0.30.0" section
- PROB-029 (Verdict aggregator)

