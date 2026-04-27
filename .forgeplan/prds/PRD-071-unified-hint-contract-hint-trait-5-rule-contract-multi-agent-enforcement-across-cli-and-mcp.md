---
created: 2026-04-27
depth: standard
domain: general
id: PRD-071
kind: prd
links:
- target: PROB-046
  relation: based_on
priority: P1
projectType: cli_tool
status: draft
title: Unified hint contract — Hint trait + 5-rule contract + multi-agent enforcement across CLI and MCP
updated: 2026-04-27
---

# PRD-071: Unified hint contract

## Problem

Forgeplan emits guidance ("what to do next") in five surfaces — CLI text output, CLI JSON output, MCP success response, CLI error message, MCP error response — but enforcement is uneven:

- 26/73 CLI commands have any hint, 47/73 have none
- 55/55 MCP tools have `_next_action` field but ~15-20 emit weak/conditional/multi-choice hints
- CLI errors: some have "Hint:" suffix, some don't; same for MCP error responses
- No single mental model for an agent reading any forgeplan output — agent must adapt per surface

When agent receives output without an actionable hint, it falls back to re-reading CLAUDE.md, re-discovering methodology, or guessing — burning tokens, sometimes hallucinating, sometimes looping. The cost compounds across thousands of tool calls per session.

Full detail in PROB-046.

## Target Users

| Persona | Need |
|---|---|
| AI agent (Claude/Cursor/Windsurf) | Receive deterministic, actionable next-step on every output to chain workflows confidently |
| CLI power user | Optional human-readable hint for self-correction, never blocking automation scripts |
| Plugin author | Same hint contract for any forgeplan-derived tool — predictable agent integration |
| CI / scripts | JSON `_next_action` field present on every response, parseable without ambiguity |

## Goals

| ID | Criterion | Metric | Target |
|---|---|---|---|
| SC-1 | Hint coverage CLI | percent of CLI commands with `Next:` marker or null-terminal | 100% (73/73) |
| SC-2 | Hint coverage MCP | percent of MCP tools with `_next_action` field (string or null) | 100% (55/55) |
| SC-3 | Hint quality CLI | percent passing 5-rule contract audit | >= 95% |
| SC-4 | Hint quality MCP | percent passing 5-rule contract audit | >= 95% |
| SC-5 | Drift prevention | integration test in CI | passes |
| SC-6 | Build | `cargo build --release` exit 0 | clean |
| SC-7 | Lint | `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings |
| SC-8 | Format | `cargo fmt --check` | 0 diff |

## Non-Goals

- Adding new business logic — hint generation is rule-based on existing response state
- LLM-generated hints — deterministic only, fast (< 1ms)
- Real-time hint streaming — hint emitted with response, not separately
- Localization (RU/EN) of hints — English only initially (CLI tradition)
- Replacing `forgeplan_health.next_actions` — that's a dashboard, this is per-call

## Functional Requirements

| ID | Priority | Requirement |
|---|---|---|
| FR-001 | Must | `forgeplan-core::hint` module exposes `Hint` struct + `HintKind` enum (Next/Or/Wait/Done/Fix) |
| FR-002 | Must | `HintEmitter` trait with default impl renders text and JSON consistently |
| FR-003 | Must | Every CLI command's text output ends with `Next: <full command>` line OR explicit terminal `Done.` line OR no hint when state is genuinely terminal |
| FR-004 | Must | Every CLI command's JSON output (when `--json`) has top-level `_next_action` key (string or null) |
| FR-005 | Must | Every MCP tool response has `_next_action` field — value passes contract |
| FR-006 | Must | CLI errors emit `Error: <reason>` line followed by `Fix: <full command>` line OR `Doc: <link>` |
| FR-007 | Must | MCP error responses include `_next_action` in error `data` field |
| FR-008 | Must | Hints are deterministic — same input state produces same output hint string (no random suggestions) |
| FR-009 | Must | Multi-choice hints use single `Next:` (primary) followed by `Or:` (fallback) — never bullet list |
| FR-010 | Must | `tests/hint_contract.rs` runs every CLI command and asserts contract |
| FR-011 | Must | `forgeplan-mcp/tests/hint_contract.rs` asserts every tool response shape |
| FR-012 | Should | Audit script `scripts/audit-hints.sh` outputs coverage metric (CI-friendly) |
| FR-013 | Should | `forgeplan_health` adds "Hint coverage" metric to dashboard |
| FR-014 | Could | Hint translation (RU) via locale config — deferred |

## Technical Approach

### Phase 0: Health check + branch
Run `forgeplan_health` (already healthy 2026-04-27 post-cleanup). Branch `feat/prd-071-hint-contract` from `dev`.

### Phase 1: Audit (measure baseline)
Script `scripts/audit-hints.sh`:
1. For each CLI subcommand: run with `--help`, run with sample args (or skip if requires workspace), capture text and JSON output
2. Parse for `Next:` marker, `_next_action` field
3. Classify: GOOD (full command), PARTIAL (fragment), MISSING, NULL_OK (terminal)
4. Output: markdown table per surface

### Phase 2: Foundation (Hint trait)
```rust
// crates/forgeplan-core/src/hint.rs
pub enum HintKind {
    Next,   // primary action: "Next: forgeplan validate PRD-001"
    Or,     // alternate when primary not applicable
    Wait,   // async/TTL state: "Wait: TTL expires in 30 min"
    Done,   // terminal success: "Done. Workflow complete"
    Fix,    // error remediation: "Fix: forgeplan score PRD-001"
}

pub struct Hint {
    pub kind: HintKind,
    pub command: Option<String>,  // full copy-pasteable command
    pub context: Option<String>,  // 1-line "why"
    pub or: Option<Box<Hint>>,    // optional secondary
}

pub trait HintEmitter {
    fn hint(&self) -> Option<Hint>;
    fn render_text(&self, hint: &Hint) -> String;
    fn render_json(&self, hint: &Hint) -> serde_json::Value;
}
```

### Phase 3: Multi-agent CLI fix (3-4 parallel)
Group 73 CLI commands by category (workspace/quality/lifecycle/multi-agent/etc.). Each agent owns 15-25 commands. Strict file ownership (one agent per `commands/<file>.rs`). Use `forgeplan_dispatch` to plan, `forgeplan_claim` to lock, `forgeplan_release` to free.

### Phase 4: MCP polish (1-2 agents)
Read `crates/forgeplan-mcp/src/server.rs`, find ~20 weak hints (audit list from Phase 1), refine to contract. Concurrent with Phase 3 (different files).

### Phase 5: Test enforcement + docs
- `tests/hint_contract.rs` — runs every CLI subcommand via `assert_cmd` + `predicates`
- `forgeplan-mcp/tests/hint_contract.rs` — verifies every tool response has field
- Update `~/.claude/skills/forge/SKILL.md` (already done in this session, just verify)
- Update `CLAUDE.md` with 5-line "Hint protocol" reference
- Create `docs/methodology/agent-protocol.md` with full contract

### Phase 6: Evidence + activate + PR
EVID-N "PRD-071 hint contract enforcement — 73 CLI + 55 MCP coverage 100%, 0 clippy, contract test passing", link to PRD-071 + PROB-046, activate, commit, push, PR.

## Dependencies

| Dependency | Type | Status |
|---|---|---|
| Forgeplan v0.24.0 binary | Internal | Released |
| `forgeplan_dispatch` (PRD-057) | Internal | Active — used for multi-agent partitioning |
| `assert_cmd` crate | External | Already in Cargo.lock |
| `predicates` crate | External | Already in Cargo.lock |

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Contract too strict — real cases don't fit | High | Audit (Phase 1) FIRST — derive contract from real patterns, then refine trait |
| Two agents touch same handler module | Medium | `forgeplan_dispatch` Jaccard threshold + claim protocol — proven in PRD-070 |
| Test slow (runs every command) | Low | Use `tempfile::TempDir` per test, parallel test execution, <60s total |
| MCP hint refinement breaks JSON-consuming clients | Medium | Hints are advisory strings, no schema change; clients ignore unknown fields |
| Drift after merge | High | Integration test in CI fails on missing hint; addition makes it impossible to ship a no-hint command |

## Acceptance Criteria

- All FRs (FR-001..011) green
- Phase 1 audit metrics published in EVID
- 73/73 CLI commands have `Next:` line or explicit `null` terminal
- 55/55 MCP tools have `_next_action` field
- CI integration test passes; new CLI command without hint fails CI
- 3 docs published (SKILL.md update, CLAUDE.md update, agent-protocol.md)

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-046 | based_on |
| PRD-070 | based_on (PR #211 surfaced inconsistency in new CLI) |
| ADR-008 | informs (self-describing tools — hint is part of self-description) |
| PRD-046 | based_on (docs sprint where hint mismatch was first noticed) |

