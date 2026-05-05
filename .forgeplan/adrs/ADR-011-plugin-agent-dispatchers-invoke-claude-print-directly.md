---
depth: standard
id: ADR-011
kind: adr
links:
- target: PRD-072
  relation: refines
status: active
title: Plugin/Agent dispatchers invoke claude --print directly
---

# ADR-011: Plugin/Agent dispatchers invoke `claude --print` directly

## Status

Active (2026-05-02). Activated together with EVID-093 spike validation. R_eff = 0.70 grade B (sole evidence EVID-093, CL3 supports, score 1.0).

## Context

Phase 6 (PRD-072 / RFC-007 / ADR-010, shipped в v0.27.0) построил Forgeplan's playbook runtime с пятью типами делегаций. Два subprocess-based варианта — `Plugin` и `Agent` — предполагали внешние бинари:

- `claude-code-plugin invoke <plugin-id> <agent>` (для `Delegation::Plugin`)
- `task-tool agent-invoke <agent-id>` (для `Delegation::Agent`)

ADR-010 явно документировал что эти бинари — гипотетические; план был либо (a) ждать пока Anthropic выпустит publicly invokable CLI, либо (b) написать shim через `anthropic-sdk-rust` direct API. Без бинарей Plugin/Agent dispatchers падали с `DispatchError::DelegateMissing` в production, делая `forgeplan playbook run brownfield-code --yes` неработающим end-to-end.

Investigation 2026-04-30 (см. `.local/spike-claude-print/findings.md`) показала что **`claude` CLI с `--print` flag уже покрывает 100% требуемой функциональности**:

- `--agent <name>` — invoke specific subagent (включая plugin-installed agents) by name
- `--print` — headless mode без TUI, output на stdout
- `--output-format json` — structured response с cost / duration / errors / session_id
- `--max-budget-usd <amount>` — built-in cost cap, halts при превышении
- `--allowedTools <tools...>` — granular per-invocation tool permissions
- `--add-dir <path>` — write permission для produces_at output directories
- Existing Claude Code login session honoured — без ANTHROPIC_API_KEY когда user уже logged in

Spike validation:

| Test | Verdict | Note |
|---|---|---|
| Agent resolution by name | 🟢 PASS | `--agent c4-code` находит installed plugin agent |
| Structured JSON output | 🟢 PASS | 17 fields включая `result`, `total_cost_usd`, `is_error` |
| File write via Write tool | 🟢 PASS | 855 bytes accurate analysis за $0.52 |
| Tool permissions | 🟢 PASS | `--allowedTools` accepts variadic Vec, NOT space-joined string |
| Existing login session | 🟢 PASS | No API key prompt при invoke |
| Budget cap enforcement | 🟢 PASS | Process halts at limit, exit 1 + partial output preserved |

## Decision

`PluginDispatcher` и `AgentDispatcher` invoke **`claude` CLI напрямую через `tokio::process::Command`** вместо несуществующих `claude-code-plugin` / `task-tool` бинарей.

Конкретные изменения:

1. `which claude` заменяет `which claude-code-plugin` / `which task-tool`. Если `claude` не на PATH — `DispatchError::DelegateMissing` с install hint "Install Claude Code from https://code.claude.com/docs/en/install".
2. Argv shape:
   ```
   claude
     --print
     --agent <step.target | step.delegate_to::Agent::name>
     --output-format json
     --max-budget-usd <budget>     # default $1.00, configurable per step
     --allowedTools <T1> <T2> ...  # variadic, separate args per tool
     --add-dir <dirname(produces_at)>  # if step.produces_at present
   ```
3. Prompt передаётся через **stdin pipe** (не как positional arg) — избегает arg-eating от variadic `--allowedTools <tools...>`.
4. Output decoding:
   - Parse stdout as JSON
   - `is_error: true` OR `api_error_status != null` → DispatchOutcome { success: false, stderr: <api_error or .result> }
   - `total_cost_usd >= max_budget_usd` → Success operator with partial budget exhausted, surface in stderr context
   - File at `produces_at` exists после invocation → success path
5. Каноническое prompt scaffolding для produces_at flow:
   ```
   <step.input.task>

   Write output to `<step.produces_at>` using the Write tool.
   ```
6. SkillDispatcher остаётся отдельным кейсом (in-process trace stub today, real registry в PRD-069). НЕ trying to map skills через `--agent` because skills и agents — different runtime concepts в Claude Code.

## Consequences

### Positive

- **Zero-install** для existing Claude Code users — `claude` already installed
- **Existing session reuse** — auth, billing, session state honoured automatically
- **Plugin/agent resolution встроен** — `~/.claude/plugins/cache/.../agents/<name>.md` находится by name, no manifest parsing нужен
- **Cost control built-in** — `--max-budget-usd` from upstream
- **Structured output** — JSON со всеми metrics
- **Stable API** — Anthropic owns the CLI contract; backward compat strong
- **Eliminates deferred items** in PRD-072 (real Plugin/Agent dispatcher integration was Wave 5 backlog)

### Negative / mitigations

- **CI/CD без интерактивного login требует ANTHROPIC_API_KEY** (env var fallback) — document in playbook prereqs
- **Lock-in на Claude Code CLI** — acceptable, alternative (writing shim ourselves) is strictly worse
- **Argument-parsing gotcha** — `--allowedTools` variadic eats positional args; solved by stdin prompt + careful arg ordering
- **Exit code disambiguation** — exit 1 может означать budget cap OR real error; required JSON parse to distinguish (already parsing для metrics, free)

### Neutral

- Slight cost per playbook run ($0.30–$5 typical range), visible to user via `--max-budget-usd` cap. Not free, but transparent.
- Updated install docs / playbook authoring guide to reflect new prereq (`claude` CLI present).

## Alternatives considered

### A — Wait for Anthropic to ship `claude-code-plugin`

Rejected per user mandate 2026-04-29: «мы не можем на такое полагаться, делаем всё сами». Passive dependency без timeline.

### B — Build shim through `anthropic-sdk-rust` direct API

Rejected:
- Duplicates `claude` CLI's plugin manifest parsing, agent resolution, session handling
- Requires `ANTHROPIC_API_KEY` always (no Claude Code session reuse)
- Bundles extra Rust dependency (`anthropic-sdk-rust`)
- Lock-in на specific SDK version
- Doesn't honour Claude Code's existing config / settings hierarchy

### C — Use third-party Rust SDK like `claude-sdk-rs`

Considered as opportunistic enhancement, NOT replacement. `claude-sdk-rs` itself wraps `claude` CLI as subprocess — same underlying mechanism. Could add as dependency for ergonomics later, but raw `tokio::process::Command` for v1 keeps surface minimal.

### D — Use `claude-code-rust` / `claurst` clean-room reimplementations

Rejected — these are TUI-focused alternatives к Claude Code itself, not headless runners. Adds entire LLM-loop rewrite as our dependency.

## Implementation plan

This ADR is the predecessor to actual code change. Implementation:

1. **Branch**: `feat/adr-011-claude-print-dispatcher`
2. **Code change** (~3-4h focused work):
   - `crates/forgeplan-core/src/playbook/dispatch/plugin_dispatcher.rs` — rewrite `dispatch()` to invoke `claude --print --agent <target>`
   - `crates/forgeplan-core/src/playbook/dispatch/agent_dispatcher.rs` — same pattern с `--agent <name>` from `Delegation::Agent`
   - Add `Step.budget_usd: Option<f64>` field (analogous to `timeout_seconds`)
   - Update SPEC-003 schema 1.1 → 1.2 (additive minor bump)
   - Add prompt-assembly helper в `dispatch::helpers` для produces_at convention
   - Update install hint text — больше не "install claude-code-plugin"
3. **Tests**:
   - Unit: prompt assembly, argv construction, JSON parse, exit-code disambiguation
   - Integration: real `claude --print` invocation в `#[ignore]` test (manual run)
4. **EVID**: Spike findings document уже exists (`.local/spike-claude-print/findings.md`), formalize as EVID-093 with structured fields (verdict: supports, congruence_level: 3, evidence_type: measurement) linked to this ADR.
5. **PR** target: `dev`. Mergeable after Phase A (PRD-073) is closed — no architectural conflict, but cleaner integration on stable base.

## Invariants

- **Single CLI binary** — `claude` MUST be the only externally invoked subprocess for plugin/agent dispatch. NO fallback to `claude-code-plugin` / `task-tool` / `anthropic-sdk-rust` shims.
- **Stdin prompt** — the prompt body MUST be piped via stdin. Passing it as a positional argument is forbidden because `--allowedTools` is variadic and would consume it.
- **Budget cap is mandatory** — every invocation MUST include `--max-budget-usd <N>` (default $1.00). Unbounded runs are not allowed.
- **JSON output mandatory** — `--output-format json` is non-negotiable. Stdout is parsed for `is_error` / `api_error_status` / `total_cost_usd`; never trust exit code alone.
- **No API key fallback in CLI mode** — when running interactively (TTY), `ANTHROPIC_API_KEY` MUST NOT be set; rely on existing Claude Code session. CI mode (no TTY) explicitly opts in via env.
- **Claude binary discoverable on PATH** — startup probe (`which claude`) is mandatory; missing binary surfaces `DispatchError::DelegateMissing` with install hint, never silent fallback.

## Rollback Plan

If `claude --print` invocation pattern proves unviable (e.g. Anthropic ships `claude-code-plugin` with a strictly better contract, or a critical security boundary in `claude` CLI breaks isolation we depend on):

1. **Short-term mitigation** — feature-flag the dispatcher (`config.dispatchers.plugin_kind = "claude-print" | "mock"`) so users can fall back to `MockDispatcher::AlwaysOk` for trace-only runs while we triage.
2. **Medium-term replacement** — switch `PluginDispatcher::dispatch()` body to invoke the new replacement binary; the surrounding contract (DispatchOutcome, timeout, env_clear, kill_on_drop) is binary-agnostic and stays.
3. **Long-term** — supersede ADR-011 with a new ADR documenting the replacement; mark this ADR `superseded`.
4. **Data preservation** — no on-disk schema changes from this decision, so rollback is purely code-side.

## Preconditions

- PRD-072 / RFC-007 / ADR-010 dispatcher trait surface stable (shipped v0.27.0).
- Phase 3c (PRD-073) merged to dev — file-first invariant is independent of dispatcher implementation but cleaner to land sequentially.
- Spike validation complete (see EVID-093) — all 6 test rows GREEN.

## Postconditions

- `forgeplan playbook run <name> --yes` end-to-end works for `Plugin` and `Agent` delegations on a workstation with `claude` on PATH.
- `forgeplan plugins doctor` reports `claude` binary status (present/missing) as part of the dispatcher health surface.
- CHANGELOG entry documents the new prereq.
- `.forgeplan/playbooks/audit.yaml` smoke test (currently deferred per HANDOFF-remaining-backlog Track 4-A8) becomes runnable.

## Affected Files

- `crates/forgeplan-core/src/playbook/dispatch/plugin_dispatcher.rs` — rewrite `dispatch()` (subprocess invocation)
- `crates/forgeplan-core/src/playbook/dispatch/agent_dispatcher.rs` — same pattern
- `crates/forgeplan-core/src/playbook/dispatch/helpers.rs` — add `assemble_prompt(step) -> String` for produces_at convention
- `crates/forgeplan-core/src/playbook/dispatch/types.rs` (or equivalent) — add `Step.budget_usd: Option<f64>`
- `crates/forgeplan-mcp/src/server.rs` — surface budget option through MCP playbook tools
- `marketplace/playbooks/audit.yaml` — set realistic budgets + tool allowlists per step
- SPEC-003 (RFC-007 schema) — version bump 1.1 → 1.2 (additive minor)
- `docs/operations/install.md` (or equivalent) — document `claude` CLI prereq
- `CHANGELOG.md` — `[Unreleased]` entry under behavior-changes

## Related Artifacts

- ADR-010 (active): predecessor — assumed phantom binaries; this ADR refines the dispatcher mechanism while preserving the rest of ADR-010's invariants (kill_on_drop, env_clear, timeout, etc.)
- PRD-072 (active): Phase 6 — this ADR closes the deferred "real subprocess invoker" item
- RFC-007 (active): Phase 6 dispatcher architecture — `Dispatcher` trait surface unchanged; only invocation mechanism inside `dispatch()` differs
- EVID-093 (TBD post-activation): formal evidence record

## Amendment 1 — 2026-05-04 — env-var override gated to test builds (PROB-050 A-14, v0.29.0)

**Context**: Phase B Wave 1 R1 audit (2026-05-03) escalated security
finding S-2 to **REQUIRED**. The `AgentDispatcher::resolve_claude_binary`
function read `$FORGEPLAN_CLAUDE_BIN` in release builds, allowing an
attacker who controls the env to redirect the `claude` subprocess to an
arbitrary binary (CWE-426 — Uncontrolled Search Path / binary
substitution). PluginDispatcher never read this env var, creating an
asymmetric attack surface.

**Amendment**: the binary-resolution contract for `claude` subprocess
invocation is hereby revised:

| Build mode | Resolution order |
|---|---|
| **Release** (any non-test build) | explicit override (`with_claude_binary`) → `$PATH` |
| **Test** (`#[cfg(test)]`) | explicit override → `$FORGEPLAN_CLAUDE_BIN` → `$PATH` |

The env-var path is preserved in test builds for fixture wiring; release
builds are now symmetric with PluginDispatcher (which never read env
vars). The same gate is applied symmetrically to
`helpers::resolve_forgeplan_binary` (`$FORGEPLAN_BIN`) — currently latent
(no production caller) but ensures the pattern is established before
Phase 7+ promotes any dispatcher to forgeplan self-invocation.

**Implementation**: `#[cfg(test)]` attribute immediately above the
`if let Ok(_) = std::env::var(...)` block in both
`crates/forgeplan-core/src/playbook/dispatch/agent_dispatcher.rs` and
`crates/forgeplan-core/src/playbook/dispatch/helpers.rs`. Inline grep-hint
comments at both gate sites name PROB-050 A-14 + CWE-426 so the security
boundary is discoverable without traversing this ADR.

**Verification**: positive tests
`resolve_claude_binary_honours_env_override_in_test_builds` (agent) and
`resolve_forgeplan_binary_respects_env_override` (helpers, strengthened
to isolate `$PATH` so the assertion exercises the env branch
deterministically) pin the test-build half. Release-build half is
enforced compile-time by Cargo's `#[cfg(test)]` semantics — unfalsifiable
from `cargo test` but verified by the security-expert audit's
case-by-case verification matrix (cargo build, `--release`, `--tests`,
doctests, integration tests, downstream consumers all confirmed to
exclude the env path).

**Closure evidence**: EVID-102 (R_eff measurement, structured fields
verdict: supports / CL3 / evidence_type: test). See PROB-050 A-14
checkbox in `.forgeplan/problems/PROB-050-phase-b-follow-ups-claude-print-dispatcher-deferrals-from-adr-011-r1-audits.md`.

**Reversibility**: high — the cfg-gate is a one-line attribute. To
restore the v0.27.0/v0.28.0 contract, remove `#[cfg(test)]` from both
gate sites and re-open A-14 with new audit cycle. Not recommended:
there is no known production use case requiring env-var override at the
binary-resolution layer (operators pin via `$PATH`; library consumers
use `with_claude_binary`).

## Sources

- `.local/spike-claude-print/findings.md` — full test matrix
- `.local/spike-claude-print/output3.json`, `test5.json` — raw spike outputs
- `.local/spike-claude-print/output-files/detect-summary.md` — qualitative output sample
- [Claude Code CLI docs](https://code.claude.com/docs/en/overview) — `--print`, `--agent`, `--allowedTools` reference
- [Subagents in SDK](https://platform.claude.com/docs/en/agent-sdk/subagents) — invocation contract


