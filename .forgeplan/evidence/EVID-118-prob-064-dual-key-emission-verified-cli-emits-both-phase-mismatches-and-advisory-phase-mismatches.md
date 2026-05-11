---
depth: tactical
id: EVID-118
kind: evidence
links:
- target: PROB-064
  relation: informs
status: active
title: PROB-064 dual-key emission verified — CLI emits both phase_mismatches and advisory_phase_mismatches
---

# EVID-118: PROB-064 — dual-key emission verified

## Summary

CLI `forgeplan health --json` now emits advisory phase mismatch data
under **both** keys: legacy `phase_mismatches` (pre-existing) and
MCP-canonical `advisory_phase_mismatches` (new alias). MCP
`forgeplan_health` is unchanged. Naming asymmetry that caused silent
breakage for agents/CI scripts moving consumers between MCP and CLI
surfaces is closed.

Implementation choice: **Option B (additive aliases)** from PROB-064
triage. Non-breaking — anyone reading `jq .phase_mismatches` keeps
working, anyone reading `jq .advisory_phase_mismatches` now works on
both surfaces. Future deprecation of the legacy key remains an option
for a major-version bump.

## Method

Test-driven verification on three axes:

1. **Unit / integration tests** — 3 new tests in
   `crates/forgeplan-cli/tests/cli_integration_test.rs`:
   - `health_json_emits_both_phase_mismatches_aliases` — both keys
     present in `health --json`, both are arrays
   - `health_json_phase_mismatches_aliases_have_identical_payload` —
     the two keys carry structurally identical values (alias contract)
   - `health_json_advisory_alias_matches_mcp_naming` — symbolic guard
     that the CLI exposes the MCP-canonical name verbatim
2. **Pipeline gates** — `cargo fmt --all -- --check` (0 diff),
   `cargo check --workspace` (0 warnings), `cargo clippy --workspace
   --all-targets -- -D warnings` (0 warnings), `cargo test --workspace
   --lib` (1965 PASS), `cargo test --test cli_integration_test`
   (107 PASS).
3. **E2E on real dogfood workspace** — built `target/debug/forgeplan`,
   invoked `forgeplan health --json`, parsed with Python, asserted
   both keys present and `legacy == canonical`. Result: True / True /
   True. `bash scripts/smoke-test.sh` PASSED.

## Findings

- Single source of truth in the emitter (`let phase_mismatches_payload`
  bound once) makes drift between the two keys impossible — they
  literally reference the same `Vec<serde_json::Value>`. No need for
  a runtime parity check.
- MCP path (`crates/forgeplan-mcp/src/server.rs:2927`) was deliberately
  left untouched. Reverse aliasing (MCP also emitting `phase_mismatches`)
  was considered and rejected — the symmetric problem doesn't exist
  in the wild yet, and adding the legacy key on the MCP side would
  permanently widen the contract for marginal value.
- Empty-workspace baseline is sufficient for the alias contract test.
  Phase tracking is opt-in per workspace config; identical-payload
  assertion holds for both empty `[]` and populated arrays by
  construction (same memory).
- The CLI text output (the rendered "Phase mismatches (N)" panel) was
  already non-empty since PROB-051 closure, so no work was needed
  there.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Linked artifacts

- **informs PROB-064** — primary problem, fix verified
- **informs PROB-063** — sibling (verdict aggregator regression);
  PROB-064 was discovered during PROB-063 verification
- **informs PROB-051** — origin of the `advisory_phase_mismatches`
  signal class (CLI L-H3 surfacing)
- **informs PROB-029** — parent anti-contradiction guarantee
  (naming inconsistency is a re-entry vector)

## Cross-surface contract reference

| Surface | Field name | Status after PROB-064 |
|---|---|---|
| CLI `forgeplan health --json` | `phase_mismatches` | emitted (legacy) |
| CLI `forgeplan health --json` | `advisory_phase_mismatches` | emitted (canonical, new) |
| MCP `forgeplan_health` | `advisory_phase_mismatches` | emitted (canonical, unchanged) |
| CLI text `forgeplan health` | rendered as "Phase mismatches (N)" panel | unchanged |



