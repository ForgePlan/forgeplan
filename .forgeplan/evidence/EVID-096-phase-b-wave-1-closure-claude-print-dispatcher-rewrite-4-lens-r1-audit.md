---
depth: standard
id: EVID-096
kind: evidence
last_modified_at: 2026-05-02T21:53:10.152816+00:00
last_modified_by: claude-code/2.1.121
links:
- target: ADR-011
  relation: informs
status: draft
title: Phase B Wave 1 closure — claude --print dispatcher rewrite + 4-lens R1 audit
---

# EVID-096: Phase B Wave 1 closure — claude --print dispatcher rewrite + 4-lens R1 audit

| Field | Value |
|-------|-------|
| Status | Active |
| Created | 2026-05-02 |
| Valid Until | 2026-08-02 (90 days) |
| Target | ADR-011 (Phase B implementation), PRD-072 (Phase 6 deferred dispatcher) |

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

Phase B Wave 1 sprint executed 2026-05-02 on branch
`feat/phase-b-real-dispatchers` (based on dev @ a696749 — post-ADR-011
landing in PR #234). Multi-agent dispatch via TeamCreate Mode A
(file-partitioned, opus model) + 4 audit lenses (security PRIORITY,
rust, code-review, architect, all opus).

**Sprint structure:**

| Phase | Agents | Output |
|-------|--------|--------|
| Pre-Wave 0 | Lead | Step.budget_usd + Step.allowed_tools fields, ClaudePrintResponse + 6 helpers + 11 unit tests, 8 dispatcher test sites bulk-patched (commit 83ebe23) |
| Wave 1A | rust-plugin (rust-pro/opus) | plugin_dispatcher.rs rewrite — 502 → 869 lines, 8 → 17 tests, validate_agent_name shared in claude_print |
| Wave 1B | rust-agent (rust-pro/opus) | agent_dispatcher.rs rewrite — 379 → 774 lines, 8 → 13 tests, ENV_GUARD as tokio::sync::Mutex, FORGEPLAN_CLAUDE_BIN env override path |
| Lead post-Wave 1 | Lead | routing.rs env-tolerance fix, clippy allow on ENV_GUARD test sites |
| Wave 1 commit | Lead | ad9bdf2 — 1153 insertions, 297 deletions across 4 files |
| Audit R1 | security-expert + rust-pro + code-reviewer + architect-reviewer (4×opus, adversarial) | 4 CRITICAL + 18 HIGH/MEDIUM/LOW findings |
| R1 fix-batch | Lead | All 4 CRITICAL closed in-flight; 3 HIGH closed; 18 items aggregated as PROB-050 follow-up tracker (commit e52e60a) |

**CRITICAL findings closed in R1 fix-batch:**

| # | Source | Issue | Fix |
|---|--------|-------|-----|
| C-1 | security | `produces_at` workspace escape via `--add-dir` (CWE-22) | `add_dir_for_produces_at` returns `Result`, rejects absolute paths and `..` components |
| C-2 | security | `allowed_tools` argv flag-injection (CWE-88) | New `validate_tool_name` regex `^[A-Z][A-Za-z0-9]{0,31}$`, bulk validation before argv |
| C-3 | code-review | PluginDispatcher argv order — `--add-dir` after `--allowedTools` consumed by variadic | Reordered to `--add-dir` first, mirroring agent_dispatcher |
| C-4 | rust + code-review | `--max-budget-usd` format divergence (`2.50` vs `2.5`) | Shared `claude_print::format_budget()` two-decimal helper |

**Surface measured:**

- 4 helpers in `claude_print.rs` rewritten / extended:
  `add_dir_for_produces_at` (signature change, returns Result),
  `validate_tool_name` (new), `validate_allowed_tools` (new),
  `format_budget` (new), `truncate_for_log` (new), `MAX_*_BYTES` constants (new)
- 2 dispatcher rewrites (PluginDispatcher 869 lines, AgentDispatcher
  774 lines) using shared helpers + argv-injection guards
- 3 commits on the sprint branch:
  - `83ebe23` — Pre-Wave 0 typed surface
  - `ad9bdf2` — Wave 1 dispatcher rewrites
  - `e52e60a` — R1 audit closure (CRITICAL fix-batch + PROB-050)

## Result

**Quantitative metrics:**

| Metric | Pre-Phase-B | Phase B Wave 1 + R1 fixes | Delta |
|--------|-------------|---------------------------|-------|
| Library tests (workspace) | 1452 (Phase 3c baseline) | 1487 | +35 |
| `claude_print` lib tests | 11 (Pre-Wave 0) | 20 | +9 |
| `plugin_dispatcher` tests | 8 (pre-rewrite) | 17 | +9 |
| `agent_dispatcher` tests | 8 (pre-rewrite) | 13 | +5 |
| `cargo clippy --workspace --all-targets --features test-helpers -- -D warnings` | clean | clean | — |
| `cargo fmt --check` | 0 diffs | 0 diffs | — |
| Audit R1 findings closed | — | 4 CRITICAL + 3 HIGH (7 in-flight) | — |
| Audit R1 findings deferred (PROB-050 tracker) | — | 20 acceptance criteria | — |

**Qualitative outcomes:**

- ADR-011 §Decision implemented: PluginDispatcher + AgentDispatcher
  invoke `claude --print` directly via tokio::process::Command
- 6 ADR-011 invariants preserved (single CLI binary, stdin prompt
  mandatory, budget cap mandatory, JSON-only decoding, no API-key
  fallback in TTY mode, claude binary discoverable)
- Argv-injection guards on BOTH user-controlled YAML strings:
  `validate_agent_name` (name + target) and `validate_tool_name`
  (allowed_tools — added in R1 fix per security audit C-2)
- Workspace-escape blocked at `--add-dir` construction
  (`add_dir_for_produces_at` rejects abs + `..`)
- Info-leak hardening parity with PRD-073 R1 H-8 pattern
  (truncate_for_log + MAX_*_BYTES caps on validator echo + render
  failure context)
- CHANGELOG entry under "Changed (behavioral)" announcing new `claude`
  CLI prereq for plugin/agent playbook steps
- PROB-050 created to track 20 deferred items as a coherent Phase B
  follow-up sprint; sibling to PROB-049 (Phase 3d typed-error
  follow-ups) — same methodology pattern of audit-driven tracker

**Architectural deferrals (PROB-050):**

- A-1 SPEC-003 1.1 → 1.2 schema bump
- A-2 ADR-010 Amendment 1 (stdin invariant relaxation)
- A-3 `#[ignore]` integration test for real `claude --print`
- A-4 `claude_print::invoke()` extraction (fan-out cohesion)
- A-6 Shared cross-file ENV_GUARD
- A-7 `pub` → `pub(crate)` lockdown
- A-8 RoutingDispatcher constructor seam (replace tautological assertion)
- A-12 typed `AgentNameError` enum
- A-14 `FORGEPLAN_CLAUDE_BIN` cfg-gate

## Interpretation

The Phase B Wave 1 sprint delivered the ADR-011 contract:
PluginDispatcher and AgentDispatcher now invoke `claude --print` with
full argv-injection hardening, structured JSON envelope decoding, and
default least-privilege tool allowlist (`Read`, `Glob`, `Grep`).
Combined with kill_on_drop, env_clear, and the existing 10 MiB stream
caps from ADR-010, the subprocess surface is auditable end-to-end.

The R1 audit caught 2 CRITICAL security issues that the Wave 1
threat-model under-scoped:

1. **`allowed_tools` argv flag-injection** — Wave 1 only validated
   `name`/`target` against argv injection. The audit identified
   `allowed_tools` as a SECOND user-controlled string flowing to argv,
   equally exploitable. Generalisation: any YAML field that becomes
   argv must have a regex guard, not just the obviously-named ones.

2. **`produces_at` workspace escape** — `--add-dir` was added to grant
   the agent write permission, but the path computation was naive
   (`workspace_root.join(rel).parent()` with no canonicalisation).
   `..` segments and absolute paths bypassed the workspace boundary.
   Fix-after-find pattern: validation gates moved up in the call
   sequence (BEFORE binary resolution / argv assembly), making the
   gate type-enforced rather than convention-enforced.

The other 2 CRITICAL items (argv order in plugin, budget format
divergence) demonstrate the cost of fan-out duplication: 80% of the
two dispatcher bodies are identical, and behavior changes done in one
place silently miss the other. PROB-050 A-4 (`claude_print::invoke()`
extraction) is the structural fix.

PRD-073 R1 audit lessons applied here: `truncate_for_log` helper
mirrors the `strip_prefix` pattern from PRD-073 H-8 — bound the
echoed user input at the type-doc level so future contributors get
the constraint by reading the helper, not the validator.

Multi-agent execution again surfaced cross-cutting concerns the lead
would have missed: 4 audit lenses found 4 CRITICAL between them, none
of which any single lens flagged on its own. Without security-priority
lensing, the `produces_at` and `allowed_tools` vectors would have
shipped.

## Congruence Level Justification

CL3 (same context, penalty 0.0) — this evidence directly measures the
sprint that ADR-011 §Implementation plan promised. The branch
`feat/phase-b-real-dispatchers` IS the ADR-011 Phase B implementation;
the audit findings, fix commits, and test counts are artifacts of that
sprint. EVID-093 (spike) measured the upstream contract pre-Phase-B;
EVID-096 (this evidence) measures the implementation closing it.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-011 | supports |
| PRD-072 | informs (Phase 6 dispatcher architecture parent) |
| EVID-093 | informs (spike validation, pre-Phase-B baseline) |
| PROB-050 | informs (Phase B follow-up tracker, created from R1 audit deferrals) |

