---
depth: standard
id: PROB-055
kind: problem
last_modified_at: 2026-05-05T20:39:06.124407+00:00
last_modified_by: claude-code/2.1.128
links:
- target: PROB-050
  relation: based_on
status: draft
title: PR-E Round 6 deferred — claude_print god-module 1066 LOC split
---

## Signal

PR-E Round 6 adversarial architectural audit (3 parallel agents, 2026-05-05)
flagged `crates/forgeplan-core/src/playbook/dispatch/claude_print.rs` as a
"god module" — **1110 LOC** as of v0.29.0 cut (1066 LOC at audit time;
+44 LOC from Round 6 closures themselves) mixing 9 responsibilities:

1. argv construction (`build_argv`)
2. environment handling
3. prompt assembly (`assemble_prompt`)
4. regex-based name validators (`validate_agent_name`,
   `validate_tool_name`, `validate_allowed_tools`)
5. byte-truncation (`truncate_for_log`)
6. JSON envelope parsing (`parse_envelope`, `ClaudePrintResponse`)
7. failure-context rendering (`render_failure_context`)
8. timeout formatting (`format_timeout_msg`)
9. cross-dispatcher test mutex (`DISPATCH_ENV_LOCK`)

Visibility is tightened well (`pub(super)` / `pub(crate)`), but cohesion is
low. The "single source of truth" rationale (closing PROB-050 A-4..A-15) is
solid, but the file will keep growing each audit round if not split.

## Constraints

- MUST NOT change observable behavior (argv shape, error message format,
  test mutex sharing).
- MUST keep `claude_print::invoke` as the single entry-point for both
  `AgentDispatcher` and `PluginDispatcher` (PROB-050 single-source-of-truth
  invariant).
- MUST NOT break any of the 22+ unit tests currently in
  `claude_print::tests`.

## Optimization Targets (1-3 max)

- **Module split**: `dispatch/claude_print/{argv.rs, envelope.rs,
  validators.rs, invoke.rs, test_lock.rs}` keeping `claude_print/mod.rs`
  as a re-export façade. Each submodule ~200 LOC, single responsibility.
- **Test relocation**: tests move alongside the function they cover
  (e.g. `validators.rs::tests` for the regex validators), not all in one
  monolith block.
- **Public API stability**: `pub(super) fn invoke`, `pub(super) fn
  build_argv`, `pub(super) fn parse_envelope`, etc. continue to resolve
  from `claude_print::*` for downstream consumers.

## Observation Indicators (Anti-Goodhart)

- Test count must stay ≥ baseline (no test deletion or weakening to
  game the file split).
- `cargo clippy --workspace --all-targets --features test-helpers
  -- -D warnings` clean before AND after split.
- Module re-export façade verified by `cargo check` of every existing
  call site (no `use` rewrites in dispatchers).

## Acceptance Criteria

- [ ] `dispatch/claude_print/` directory replaces single-file module.
- [ ] 5 submodules: `argv.rs`, `envelope.rs`, `validators.rs`,
  `invoke.rs`, `test_lock.rs`.
- [ ] `mod.rs` is a re-export façade ≤ 50 LOC: `pub use argv::*;` etc.
- [ ] Each submodule LOC ≤ 350 (no new god-module).
- [ ] Tests collocated: `validators::tests` covers regex validators,
  `argv::tests` covers `build_argv`, etc.
- [ ] Existing call sites in `agent_dispatcher.rs` +
  `plugin_dispatcher.rs` compile **unchanged** (no `use` rewrites).
- [ ] CHANGELOG entry under **Refactor** section.

## Refs

- PR-E Round 6 audit (2026-05-05): architect-reviewer agent MED-2
- CHANGELOG.md (v0.29.0): "Deferred to v0.30.0" section
- PROB-050 A-4 (single-source-of-truth invariant)

