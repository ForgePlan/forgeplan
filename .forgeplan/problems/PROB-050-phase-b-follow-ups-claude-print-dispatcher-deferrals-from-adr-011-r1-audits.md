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
- [ ] **A-14 (security H-6 + 2026-05-03 audit S-2)**: gate
      `FORGEPLAN_CLAUDE_BIN` env override behind `#[cfg(test)]` —
      **REQUIRED** (audit S-2 escalation: documentation alone is not a
      mitigation for an env-injection / binary-substitution vector
      CWE-426). Today: AgentDispatcher honors it in release builds;
      PluginDispatcher does not read it at all (mismatched surface
      empirically confirmed 2026-05-03). Fix: cfg-gate + restore parity by
      removing env-var path entirely from production builds.
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

### Real-E2E discovered (2026-05-03 NOTE-049 / PR 1)

Items added based on real `claude --print` invocation findings (см.
`docs/operations/phase-b-real-e2e-2026-05-03.md`). Empirically validated
on `claude` 2.1.126 + dev binary built from `5e08b4d`.

- [x] **A-3 closure (proven)**: real `claude --print` invoked end-to-end
      from BOTH PluginDispatcher and AgentDispatcher with argv recording
      wrapper. Argv shape matches ADR-011 §Decision verbatim. JSON envelope
      decoded successfully on success and failure paths. argv injection
      guard rejects malicious agent name in 0.01s without spawning. Total
      cost: ~$1.13 across 5 successful invocations. Evidence: EVID-097
      (TBD) + ops doc R-6a-* sections.
- [ ] **A-21 (NEW, real-E2E F-RUNTIME-1)**: playbook discovery uses
      cwd-relative search (`.forgeplan/playbooks/` → `marketplace/playbooks/`
      → plugin dirs). Built-in `marketplace/playbooks/` therefore inaccessible
      from arbitrary user workspaces — only forgeplan-repo callers see them.
      Bundle built-ins into binary OR resolve from a known global location
      (e.g. `~/.config/forgeplan/playbooks/`).
- [x] ~~**A-22 (NEW, real-E2E F-RUNTIME-2)**~~: **RETRACTED 2026-05-03 audit
      C-1**. Original observation `EXIT_CODE=0` was a `tee` pipeline
      artefact (`zsh` без `pipefail`, `$?` reflected tee not forgeplan).
      `commands/playbook.rs:473` already does `exit(1)` on `failed > 0`;
      `playbook.rs:370-376` does `exit(2)` on resolve failure. Re-verified
      with `set -o pipefail`: H5 → exit 1, H4 → exit 1, missing playbook
      → exit 2. **Methodological lesson** (own learning, not a CLI fix):
      future shell-driven exit-code testing must use `set -o pipefail`
      OR capture `${PIPESTATUS[0]}` BEFORE piping through `tee`. No
      action needed on production code.
- [ ] **A-23 (NEW, real-E2E F-RUNTIME-3 + 2026-05-03 audit S-1)**:
      `marketplace/playbooks/brownfield-docs.yaml` header claims "fails
      with `DispatchError::DelegateMissing` (step 1)" when `forge-docs-miner`
      skill missing — but `SkillDispatcher` is an intentional v1 stub
      (Phase 6 Wave 5+ TBD per `skill_dispatcher.rs:24-50`) that always
      returns `success: true` without actual invocation. **Audit S-1
      escalation**: the silent-skill-no-op pattern violates fail-safe
      design (CWE-754 / CWE-755) — a release-style playbook with a
      `verify-signing` skill step would silently green-build. **Strongly
      preferred fix (option c, NEW)**: change SkillDispatcher v1 stub to
      return `success: false` with `DispatchError::DelegateMissing`-like
      reason (`"skill registry not yet implemented (Phase 6 Wave 5+);
      treat skill steps as failures until then"`) — this is fail-safe
      behavior pending Wave 5. Alternative options (a) update YAML
      header, (b) land Wave 5 — only acceptable if (c) deemed too
      breaking.
- [ ] **A-24 (NEW, real-E2E F-RUNTIME-5)**: dev binary built from `dev`
      branch returns the same `forgeplan --version` string (`0.27.0`) as
      the brew-installed last-released binary. Users (and bug-reporters)
      cannot distinguish runtime. Append git SHA + build-time to version
      output for non-release builds (`0.27.0+5e08b4d-dev`).
- [ ] **A-25 (NEW, real-E2E F-RUNTIME-6)**: `claude --print --max-budget-usd N`
      enforces budget **post-hoc**: real spend may exceed `N` by 2-5×
      (measured: `N=$0.05` produced `total_cost_usd=$0.20184575` before
      `subtype: error_max_budget_usd` returned). Document this in ADR-011
      §Decision and `claude_print.rs` module docs. Optionally expose a
      "hard kill on threshold" wrapper if Anthropic CLI gains preemptive
      enforcement.

A-14 empirical confirmation: `PluginDispatcher::resolve_binary` does NOT
read `$FORGEPLAN_CLAUDE_BIN` — verified by real-E2E (Plugin run bypassed
recording wrapper set via env, required PATH-prepend symlink instead).
This is the divergence A-14 calls out; ops doc F-RUNTIME-7 cross-references.

- [ ] **A-26 (NEW, 2026-05-03 audit S-3 + C-4)**: methodology hardening
      for future real-E2E sprints — (1) recording dirs MUST be created
      with `mktemp -d -t forgeplan-e2e-XXXXXX` (mode 700) rather than
      fixed `/tmp/phase-b-e2e-recordings/` (CWE-377 + CWE-532 — leak of
      prompts/responses on shared CI runners); (2) `STDIN_LOG` should be
      gated behind explicit `--capture-stdin` flag когда run может
      обрабатывать sensitive data; (3) every shell-driven exit-code test
      MUST use `set -o pipefail` или `${PIPESTATUS[0]}` (lesson learned
      from A-22 retraction); (4) extend H1/H2/H_PLUGIN coverage to
      include malformed JSON envelope, HTTP 5xx, signal exit, timeout
      branches (currently only happy + budget-error envelopes verified
      end-to-end — failure-path JSON decode still fake-script only).
      Items (1)-(3) are methodology-doc only; item (4) overlaps with
      A-11 + A-16 and may be folded there.

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
