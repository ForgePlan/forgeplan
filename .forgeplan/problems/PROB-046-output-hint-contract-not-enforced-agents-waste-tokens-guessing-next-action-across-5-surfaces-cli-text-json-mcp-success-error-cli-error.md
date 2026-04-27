---
depth: standard
id: PROB-046
kind: problem
links:
- target: PRD-070
  relation: informs
status: active
title: Output hint contract not enforced — agents waste tokens guessing next-action across 5 surfaces (CLI text/JSON, MCP success/error, CLI error)
---

# PROB-046: Output hint contract not enforced

## Signal

E2E spot-check 2026-04-27 of 6 newly-shipped CLI commands (PRD-070) showed inconsistent hint behavior:

| Command | Has actionable hint? | Quality |
|---|---|---|
| `forgeplan validate` (existing) | NO | "Result: PASS (with warnings)" — terminal, no next |
| `forgeplan phase` | YES | "To start tracking: `forgeplan phase-advance PRD-001 --to shape`" |
| `forgeplan phase-advance` | PARTIAL | "suggested next: adi" — bare word, not full command |
| `forgeplan activity-stats` | PARTIAL | "Try a longer window: `--since-hours 720`" — fragment, not full command |
| `forgeplan dispatch` | NO | reasoning lines but no "what to do next" |
| `forgeplan claims` | NO | "No active claims. Workspace is free for any agent" — terminal w/o next |
| `forgeplan undo-last` | YES | error has "Hint: expand window with `--within-hours 720`, or inspect..." |

Quantified estimate (extrapolated to 73 CLI commands + 55 MCP tools):
- 26/73 CLI commands have any hint (~36%)
- 47/73 CLI commands have NO hint (~64%)
- 55/55 MCP tools have `_next_action` field but ~15-20 have weak/conditional/multi-choice hints

Cost: agent reading no-hint output spends extra tokens reasoning "what next?" — re-reads CLAUDE.md, re-discovers methodology, sometimes guesses wrong, leading to loops, hallucinations, or stops productive work.

Cross-surface inconsistency: same workflow accessed via CLI vs MCP gets different guidance quality. Agent must learn two mental models.

## Constraints

- Backward compat — `_next_action` JSON field is additive; CLI text hints must not interfere with grep/jq scripts (use `Next:` marker line, do not change stdout structure)
- No blocking output on hint computation — hint must be deterministic from response data, not require extra LLM call
- No LLM for generating hints — all hints rule-based from response state
- Test-first — contract without enforcement test will drift within 1 release
- Multi-agent friendly — fixes must be partitionable by files without conflict

## Optimization Targets

1. Hint coverage 100% across all 5 output surfaces (CLI text, CLI JSON, MCP success, CLI error, MCP error)
2. Hint quality: each hint passes 5-rule contract (PRESENCE, ACTIONABILITY, DETERMINISM, CONDITIONALITY, CONSISTENCY)
3. Drift prevention: integration test fails CI if any new command or tool ships without contract-compliant hint

## Observation Indicators (Anti-Goodhart)

- Do not optimize "number of hints" — better one `null` (terminal) than fake-hint "all done!"
- Do not optimize hint length — shorter is better, prefer 1-line plus optional Or-suffix
- Do not duplicate hint per scenario — same state means one hint
- Watch for hint paralysis — if agent ignores hints, problem is not in format but in trust (specificity, actionability)

## Acceptance Criteria

- [ ] `crates/forgeplan-core/src/hint.rs` exists with `Hint` struct, `HintKind` enum (Next, Or, Wait, Done, Fix), `HintEmitter` trait
- [ ] `crates/forgeplan-cli/tests/hint_contract.rs` integration test runs every CLI command, asserts presence of `Next:` marker or explicit terminal status
- [ ] `crates/forgeplan-mcp/tests/hint_contract.rs` integration test asserts every tool response has `_next_action` field (string or null)
- [ ] All 73 CLI commands emit contract-compliant hints (validated by audit script in Phase 1)
- [ ] All 55 MCP tools have refined hints meeting 5-rule contract
- [ ] CLI error pattern: `Error: <reason>` then `Fix: <full command>`
- [ ] MCP error pattern: error response includes `_next_action` in data field
- [ ] `~/.claude/skills/forge/SKILL.md` has "Reading forgeplan output" section with good and bad examples
- [ ] `CLAUDE.md` has short "Hint protocol" reference
- [ ] `docs/methodology/agent-protocol.md` published with full contract and table of hint kinds
- [ ] PR merged to dev

## Blast Radius

- High: every CLI command file `crates/forgeplan-cli/src/commands/*.rs` (73 files)
- Medium: `crates/forgeplan-mcp/src/server.rs` (55 tool handlers, ~20 need polish)
- Medium: `crates/forgeplan-core/src/hint.rs` (new module, ~200 LOC)
- Low: docs (3 files: SKILL.md, CLAUDE.md, agent-protocol.md)
- Low: tests (2 new integration test files)

## Reversibility

High — all changes additive. CLI hints add new lines (do not change stdout structure). MCP `_next_action` is already an existing field (just normalizing values). If contract too strict, relax test assertions; existing functionality unaffected.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-070  | informs (PR #211 surfaced hint inconsistency in new CLI commands) |
| PRD-046  | based_on (docs sprint where hint mismatch was first noticed) |
| ADR-008  | informs (self-describing tools — hint is part of self-description) |
| PRD-071  | informs (solution: trait + multi-agent enforcement) |


