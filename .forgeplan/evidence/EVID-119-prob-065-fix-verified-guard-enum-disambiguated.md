---
depth: tactical
id: EVID-119
kind: evidence
links:
- target: PROB-065
  relation: informs
status: active
title: PROB-065 fix verified — guard enum disambiguated
---

# EVID-119 — PROB-065 fix verified: `forgeplan_guard` enum disambiguated from artifact-phase enum

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Summary

PROB-065 documented that the MCP `forgeplan_guard` argument enum `PhaseKind`
(`idle/routing/shaping/coding/evidence/pr` — methodology session machine)
lexically overlaps the artifact lifecycle phase enum used by
`forgeplan_phase_advance` (`shape/validate/adi/code/test/audit/evidence/done`).
Both contain a valid `evidence` variant with different runtime meaning, so a
caller copying the JSON shape from one tool to the other passes schema
validation while silently hitting the wrong state machine.

This evidence pack records the **Option A** fix from PROB-065 §Decision
(rename CLI/MCP-visible enum + sharpen tool descriptions) shipped on branch
`fix/prob-065-guard-enum` in worktree `/Users/explosovebit/Work/fpl-w5-p065`.

## Fix scope

- `crates/forgeplan-mcp/src/server.rs`
  - Rust type rename: `enum PhaseKind` → `enum SessionPhaseKind`
    (with doc-comment cross-referencing artifact-lifecycle `Phase`).
  - `impl PhaseKind { fn as_str }` → `impl SessionPhaseKind { fn as_str }`
    (helper retained, still `#[allow(dead_code)]`).
  - MCP arg rename: `GuardParams.target_phase: PhaseKind` →
    `target_session_phase: SessionPhaseKind` with
    `#[serde(alias = "target_phase")]` retaining backward compatibility for
    the legacy wire name.
  - `forgeplan_guard` tool description rewritten to (a) explicitly name
    «methodology session phase», (b) enumerate the variants, and (c)
    cross-reference `forgeplan_phase_advance` for the artifact-lifecycle
    machine plus its variants.
  - `forgeplan_guard` tool annotation title:
    `"Phase Transition Check"` → `"Session Phase Transition Check"`.
  - `forgeplan_phase_advance` tool description amended to (a) name
    «artifact lifecycle phase», (b) cross-reference `forgeplan_guard`, and
    (c) flag PROB-065 inline.
  - `forgeplan_phase_advance` annotation title:
    `"Advance Phase"` → `"Advance Artifact Lifecycle Phase"`.
- `crates/forgeplan-mcp/tests/common/mod.rs`
  - New helper `McpFixture::peer_list_all_tools()` returns the paginated
    `tools/list` catalog through the in-process JSON-RPC peer with a 15-s
    timeout (mirrors the panic-with-tool-name policy of `call_tool_json`).
- `crates/forgeplan-mcp/tests/integration_full_coverage.rs`
  - `c43_forgeplan_guard_smoke`: redundant inline disambiguation comment
    removed (now lives in the user-facing schema description); test
    upgraded to exercise the canonical `target_session_phase` argument.
  - New regression test
    `guard_target_session_phase_disambiguated_from_artifact_phase` pins
    three contracts:
      1. `forgeplan_guard` accepts `{"target_session_phase": "evidence"}`
         without `is_error`.
      2. `forgeplan_guard` accepts the legacy `{"target_phase": "evidence"}`
         alias without `is_error` (serde-alias backward compat).
      3. The descriptions returned from `tools/list` for both
         `forgeplan_guard` and `forgeplan_phase_advance` contain the
         disambiguation phrases (`"session phase"`/`"forgeplan_phase_advance"`
         on the guard side; `"artifact lifecycle phase"`/`"forgeplan_guard"`
         on the phase-advance side).

## Pipeline gate evidence

Executed in worktree `/Users/explosovebit/Work/fpl-w5-p065` on branch
`fix/prob-065-guard-enum`:

- `cargo fmt --check` — 0 diff.
- `cargo check --workspace` — 0 warnings, all three crates compiled.
- `cargo clippy --workspace --all-targets -- -D warnings` — 0 warnings.
- `cargo test -p forgeplan-mcp` — 60 (full_coverage) + 3 (server_capabilities)
  + 4 (soft_delete_integration) passed; 0 failed. New
  `guard_target_session_phase_disambiguated_from_artifact_phase` test
  PASSED.
- `cargo test --workspace --lib` — 1637 passed, 1 pre-existing flaky test
  (`playbook::dispatch::plugin_dispatcher::tests::dispatch_with_produces_at_includes_add_dir`)
  failed due to 5-s timeout under parallel load; re-run in isolation PASSED.
  Unrelated to PROB-065 surface.
- `bash scripts/smoke-test.sh` — all 18 operations PASSED on a fresh
  tempdir workspace.

## Backward compatibility

- Existing callers that emit `target_phase` continue to deserialize
  successfully because `#[serde(alias = "target_phase")]` is declared on
  the renamed field. The JSON schema published by rmcp will advertise the
  new canonical name `target_session_phase`; legacy schema introspectors
  still see acceptance.
- No CLI surface was touched — `forgeplan guard` does not exist as a CLI
  subcommand in v0.30.0 (PROB-065 §Decision A.5 mentions a deprecation
  alias for a future CLI surface; this fix lives strictly on the MCP side
  consistent with the immediately visible silent-confusion risk).

## Acceptance-criteria coverage (PROB-065 §Acceptance)

- **A-1** ✅ `forgeplan_guard` description contains «session phase» and
  cross-references `forgeplan_phase_advance` verbatim.
- **A-2** ✅ `forgeplan_phase_advance` description contains «artifact
  lifecycle phase» and cross-references `forgeplan_guard` verbatim.
- **A-3** ✅ Wire-format parameter renamed to `target_session_phase`;
  `target_phase` retained as serde alias for backward compatibility.
- **A-4** ✅ Regression test
  `guard_target_session_phase_disambiguated_from_artifact_phase` asserts
  that both tool descriptions contain the disambiguation phrases.
- **A-5** ✅ Redundant inline comment removed from `c43_forgeplan_guard_smoke`;
  the smoke test now exercises the canonical argument name.

## Why CL3

The change-and-test pair are in the same bounded context (MCP tool
schema + `tools/list` contract). The regression test queries the exact
runtime artefact that an MCP client would parse (`Tool.description`
string), so the evidence's signal is the very thing the fix is supposed
to repair — no analogical leap, no environment translation.

verdict: supports — the test green-lights the disambiguation rule;
running the test under any reverted code would surface the gap
immediately because both contains-checks would fail.

## Links

- Informs: PROB-065 (this PROB's resolution).
- Related: PRD-052 (methodology session phase machine), PRD-051 /
  RFC-006 (artifact lifecycle phase machine), PROB-064 (sister
  dual-key emission fix, same cross-surface-asymmetry class).
- Discovered by: Wave 4A security audit, v0.31.0 sprint
  (`sprint-v031-cleanup`), MED-2 finding.

## Reproducibility

```sh
# In worktree /Users/explosovebit/Work/fpl-w5-p065 on
# fix/prob-065-guard-enum:
cargo test -p forgeplan-mcp guard_target_session_phase_disambiguated_from_artifact_phase
cargo test -p forgeplan-mcp c43_forgeplan_guard_smoke
```



