---
depth: standard
id: PROB-050
kind: problem
last_modified_at: 2026-05-02T21:49:30.728979+00:00
last_modified_by: claude-code/2.1.121
links:
- target: ADR-011
  relation: based_on
status: draft
title: Phase B follow-ups — claude --print dispatcher deferrals from ADR-011 R1 audits
---

# PROB-050: Phase B follow-ups — claude --print dispatcher deferrals from ADR-011 R1 audits

## Signal

ADR-011 Phase B Wave 1 shipped PluginDispatcher / AgentDispatcher rewrites
to invoke `claude --print` (commit ad9bdf2). 4 specialized audit lenses
(security, rust, code-review, architect, all opus) returned 4 CRITICAL
+ 18 HIGH/MEDIUM findings. CRITICAL findings (path traversal in
`produces_at`, argv flag-injection in `allowed_tools`, plugin argv
order, budget format divergence) were closed in-flight before PR. The
remaining HIGH/MEDIUM items are coherent enough to track as a single
Phase B follow-up sprint rather than orphan TODO comments scattered
across the dispatcher modules.

`TODO(PROB-050)` markers in code surface this PROB via grep.

## Constraints

- MUST NOT regress the security boundary established by R1 fixes (path
  validation, allowed_tools validation, argv order, format_budget shared).
- MUST keep `claude --print` as the only invocation mechanism (ADR-011
  invariant — no fallback to fictional binaries).
- MUST run audit (4+ agents, security-priority) on the Phase B follow-up
  PR — same rigor as Phase B Wave 1.

## Optimization Targets (1-3 max)

- **Spec / methodology hygiene**: SPEC-003 1.1 → 1.2 bump,
  ADR-010 Amendment 1 documenting the stdin-pipe relaxation,
  `#[ignore]` integration test for real `claude --print`.
- **Code organization**: extract `claude_print::invoke()` so Plugin and
  Agent dispatcher bodies stop duplicating the 9-step recipe.
- **Test isolation**: shared cross-file ENV_GUARD between Plugin and
  Agent dispatcher tests.

## Observation Indicators (Anti-Goodhart)

- Test count must stay ≥ baseline at each sub-PR (no test deletion to
  game the file split).
- `cargo clippy --workspace --all-targets -- -D warnings` clean before
  AND after each Phase B follow-up sub-PR.
- `forgeplan health`: blind_spots / orphans / stale stays at 0.

## Acceptance Criteria

Items pulled from R1 audit reports (security / rust / code-review /
architect, all carry `TODO(PROB-050)` markers in code where applicable):

- [ ] **A-1 (architect C-1)**: SPEC-003 schema bump 1.1 → 1.2 with
      `Step.budget_usd` + `Step.allowed_tools` + `Step.timeout_seconds`
      rows + version section update.
- [ ] **A-2 (architect H-3)**: ADR-010 Amendment 1 documenting that the
      stdin invariant `Stdio::null()` is relaxed to `Stdio::piped()` for
      ADR-011 prompt-pipe path; closure-after-write preserves the
      no-interactive-injection guarantee.
- [ ] **A-3 (architect M-2 + code-review H-2)**: open
      `#[ignore] e2e_claude_print_argv_shape_real_binary` integration
      test (per dispatcher) gated on `CLAUDE_BIN_AVAILABLE=1`.
- [ ] **A-4 (architect H-1 + rust C-1 + code-review C-2)**: extract
      `claude_print::invoke(slug, step, workspace, binary, default_timeout)
      -> Result<DispatchOutcome, DispatchError>` so Plugin and Agent
      dispatchers reduce to (a) variant unpack, (b) compute slug, (c)
      call invoke. Closes the fan-out cohesion problem.
- [ ] **A-5 (architect H-4)**: promote `which_in_path` from 3 duplicate
      copies to `pub(super) fn` in `helpers.rs`.
- [ ] **A-6 (architect H-5 + code-review H-6)**: shared
      `pub(super) static DISPATCH_ENV_LOCK: tokio::sync::Mutex<()>` in
      `claude_print.rs`; both dispatcher test modules consume it
      (cross-file PATH-mutation race).
- [ ] **A-7 (architect M-1)**: tighten `claude_print` API surface from
      `pub` to `pub(super)` for helpers + `pub(crate)` for
      `ClaudePrintResponse` / `DEFAULT_*`. Closes external-coupling-to-
      claude-CLI-private-shape risk.
- [ ] **A-8 (architect M-4)**: replace tautological `result.is_err() ||
      result.is_ok()` routing assertions with constructor-seam injection
      (`RoutingDispatcher::with_inner_dispatchers(...)`) so routing tests
      assert deterministic `DelegateMissing` regardless of host.
- [ ] **A-9 (rust H-1)**: empirically re-check whether
      `clippy::await_holding_lock` fires on `tokio::sync::MutexGuard` in
      this toolchain; if not, remove the 6 dead `#[allow]` attrs.
- [ ] **A-10 (rust H-2)**: drop `pub` from `AgentDispatcher` fields
      (`workspace_root`, `claude_binary`, `default_timeout`) to match
      `PluginDispatcher` private encapsulation.
- [ ] **A-11 (rust H-3)**: factor `parse_envelope(stdout: &[u8]) ->
      Result<ClaudePrintResponse, ParseDiag>` and `format_timeout_msg(label,
      duration)` into `claude_print.rs`. Single source of truth for both
      message and parse semantics (currently Plugin uses no `.trim()`,
      Agent uses `.trim()`; Plugin formats timeout in seconds, Agent in
      Debug repr).
- [ ] **A-12 (rust M-1)**: typed `AgentNameError` enum (Empty / TooLong /
      LeadingDash / BadChar / LeadingNonAlpha) instead of stringly-typed
      `Result<(), String>`.
- [ ] **A-13 (rust L-1)**: add `since = "0.28.0"` to plugin
      `with_task_tool` deprecation; align with agent variant.
- [ ] **A-14 (security H-6)**: gate `FORGEPLAN_CLAUDE_BIN` env override
      behind `#[cfg(test)]` OR document as test-only with explicit
      provenance warning. Today either dispatcher honours it; mismatched
      surface (plugin doesn't read it, agent does).
- [ ] **A-15 (security M-3, code-review M-1)**: factor argv builder
      (`claude_print::build_argv(slug, step) -> Vec<String>`) so
      argv-shape tests live in `claude_print.rs` and don't need fake
      binaries.
- [ ] **A-16 (code-review H-3)**: parameterized test of `api_error_status`
      strings (timeout, server_error, rate_limited); empty-stdout case;
      budget-cap-mid-flight case (`total_cost_usd >= max_budget_usd`
      with `is_error: false`).
- [ ] **A-17 (code-review H-4)**: validate_agent_name rejection cases
      battery for AgentDispatcher (currently 1 case, Plugin has 4).
- [ ] **A-18 (code-review M-2)**: replace `contains(token)` argv assertion
      in plugin_dispatcher with by-index assertion (mirror agent_dispatcher
      pattern that captures argv to tempfile, asserts `lines[0] == "--print"`).
- [ ] **A-19 (code-review M-6)**: switch plugin_dispatcher tests from
      `std::env::temp_dir()` + manual cleanup to `tempfile::tempdir()`
      RAII pattern (matches agent_dispatcher).
- [ ] **A-20 (rust M-2 + code-review L-1)**: promote magic preview lengths
      to symbolic `pub(crate) const PREVIEW_*: usize` in `claude_print.rs`.
      Partly addressed in R1 fix (added `MAX_PREVIEW_BYTES`,
      `MAX_VALIDATOR_ECHO_BYTES`) — sweep remaining hardcoded `200`
      / `500` to use these constants everywhere.

## Blast Radius

- `forgeplan-core::playbook::dispatch::*` (PluginDispatcher,
  AgentDispatcher, claude_print, helpers, routing) — internal refactors
  landing as small independent PRs.
- SPEC-003 schema bump touches `.forgeplan/specs/` (doc-only).
- ADR-010 amendment touches `.forgeplan/adrs/` (doc-only).
- `forgeplan-cli` and `forgeplan-mcp` unaffected — consume dispatchers
  via the unchanged `Dispatcher` trait.

## Reversibility

medium — Phase B follow-ups are individually reversible refactors. The
two notable behavior changes (typed `AgentNameError`,
`FORGEPLAN_CLAUDE_BIN` cfg-gate) are downstream-visible but additive
(new variants don't break match arms; cfg-gate only narrows a test/dev
hook).

---

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-011 | based_on (parent — closes Phase B Wave 1, this is the open-work follow-up) |
| PRD-072 | informs (Phase 6 dispatcher architecture parent) |
| EVID-093 | informs (spike validation, real-binary contract) |
| PROB-049 | informs (sibling — Phase 3d typed-error follow-ups; same methodology pattern of audit-driven follow-up tracker) |
| ADR-010 | informs (Amendment 1 work item — A-2) |
| SPEC-003 | informs (schema bump work item — A-1) |
