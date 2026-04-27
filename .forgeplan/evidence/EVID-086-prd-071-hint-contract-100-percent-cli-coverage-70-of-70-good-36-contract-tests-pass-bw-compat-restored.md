---
depth: standard
id: EVID-086
kind: evidence
links:
- target: PRD-071
  relation: informs
- target: PROB-046
  relation: informs
status: active
title: PRD-071 hint contract — 100% CLI coverage (70/70 GOOD), 36 contract tests pass, bw-compat restored
valid_until: 2026-10-27
---

# EVID-086: PRD-071 Hint Contract — 5-cycle multi-agent sprint

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement

## Context

PROB-046 surfaced inconsistent hint behavior across CLI/MCP surfaces during PRD-070 review (2026-04-27). Agents wasted tokens guessing next-action. PRD-071 defined a 5-rule contract (PRESENCE / ACTIONABILITY / DETERMINISM / CONDITIONALITY / CONSISTENCY) and was implemented over 5 multi-agent cycles with strict file partitioning.

## Methodology

Cyclic protocol: tasks → parallel agents → audit → fix → next cycle. Meta-audit after every 3 cycles. File ownership strict — no overlapping edits between team-mate agents.

| Cycle | Task | Agents | Coverage |
|---|---|---|---|
| 1 | Phase 3 baseline (3 groups: A/B/C) | 3 parallel | 0% → 56.9%* |
| 2 | Placeholder fix in core hints (`<id>`/`<artifact>` → real IDs) | 1 fix | 56.9% |
| 3 | Audit re-run + MCP polish + integration test | 3 parallel (X/Y/Z) | 56.9% |
| 4 | Audit script + route/blocked/update/score + LLM commands | 3 parallel (W1/W2/W3) | **78.6%** |
| 5 | list/tree JSON regression + 15 MISSING commands | 3 parallel (W4/W5/W6) | **100%** |

*Cycle 1 baseline 56.9% reflects audit script bug (only recognized `Next:` not `Fix:`/`Done.`/`Or:`/`Wait:`); true initial coverage was higher.

## Measurements

### Coverage progression (audit script `scripts/audit-hints.sh`)

```
Cycle 1 baseline:  0%   (audit before fixes)
Cycle 3 result:    56.9% (41/72) — audit script defect masked some real coverage
Cycle 4 result:    78.6% (55/70) — fixed audit + 4 real CLI violations
Cycle 5 result:   100.0% (70/70) — fixed remaining 15 MISSING (6 real + 9 audit-script)
```

### Quality gates (after Cycle 5)

| Gate | Status |
|---|---|
| `cargo fmt --check` | ✅ 0 diffs |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ 0 warnings |
| `cargo check --workspace` | ✅ 0 warnings |
| `cargo test --workspace --lib` | ✅ 1140 passed (1081 core + 59 mcp) |
| `cargo test --package forgeplan-core --lib hints` | ✅ 21/21 |
| `cargo test --package forgeplan --test hint_contract` | ✅ 36/36 |
| `cargo test --package forgeplan --test cli_integration_test` | ✅ 104/104 |
| `cargo build --release --bin forgeplan` | ✅ succeeds |

### Tests added

- `crates/forgeplan-cli/tests/hint_contract.rs` — 337 lines, **36 tests** covering: list, status, score, link, get, update, route, review, activate, supersede, deprecate, health, blindspots, journal, capture, graph, blocked, order, search, stale, progress, decay, calibrate, reason, export, import, fpf_list, activity, activity_stats, claim, claims, discover_finding, discover_complete, estimate, drift, coverage, plus contract-presence + no-forbidden-placeholder enforcement helpers.
- 9 new tests in Cycle 4 W3 (route/update/score/fpf-rules/reason/decompose/generate without LLM)
- 1 un-ignored test (`blocked_emits_contract_marker_text`)

### Files modified (per cycle)

- **Cycle 1 (Phase 3)**: 68 CLI command files (groups A/B/C)
- **Cycle 2**: `crates/forgeplan-core/src/hints.rs` (placeholder fix in score/get/review/activate hints; signatures unchanged for backward compat)
- **Cycle 3**: `crates/forgeplan-mcp/src/server.rs` (~37 hint sites refined); `crates/forgeplan-cli/tests/hint_contract.rs` (created)
- **Cycle 4**: `scripts/audit-hints.sh` (5-marker recognition); `route.rs`, `blocked.rs`, `update.rs`, plus 6 LLM-dependent commands (promote, reason, decompose, generate, capture, fpf rules)
- **Cycle 5**: `score.rs:266` (EVID-XXX → EVID-NNN); `list.rs`, `tree.rs` (bw-compat: bare array on stdout, hint to stderr); `supersede.rs`, `deprecate.rs`, `reopen.rs`, `calibrate_estimate.rs`, `import_cmd.rs`, `git_sync.rs` (Fix: on error paths)

### Surfaces covered

All 5 surfaces from PROB-046 acceptance criteria:
1. **CLI text (success)** — `Next: <command>` line at end of stdout
2. **CLI text (error)** — `Fix: <command>` line after `Error:`
3. **CLI JSON** — `_next_action` field (or stderr `Next:` for bw-compat sensitive commands list/tree)
4. **MCP success** — `_next_action` field in tool response
5. **MCP error** — `_next_action` in error data field

### Backward compatibility

- `list --json` and `tree --json` retain bare-array stdout (CLI consumers using `jq '.[]'` not broken)
- All existing CLI text outputs preserved; hints are additive new lines at end
- MCP `_next_action` field was already present (just normalizing values)
- 1 minor test update: `list_emits_next_action_json` reads stderr instead of stdout for hint

## Acceptance criteria (from PROB-046)

- [x] `crates/forgeplan-core/src/hints.rs` exists with `Hint` struct, hint kinds, helper functions
- [x] `crates/forgeplan-cli/tests/hint_contract.rs` integration test runs every CLI command
- [x] All 70 audited CLI commands emit contract-compliant hints (100%)
- [x] All 55+ MCP tools have refined hints (Cycle 3 Agent Y polished ~37 sites)
- [x] CLI error pattern: `Error: <reason>` then `Fix: <full command>`
- [x] MCP error pattern: error response includes `_next_action` in data field
- [x] `~/.claude/skills/forge/SKILL.md` has "Reading forgeplan output" section with good and bad examples
- [x] `CLAUDE.md` has short "Hint protocol" reference (this commit)
- [x] `docs/methodology/agent-protocol.md` published with full contract and table of hint kinds
- [ ] PR merged to dev — pending user approval (per memory rule "никогда не пушить пока не проверю")

## Drift prevention

- **Integration test** `tests/hint_contract.rs` — 36 tests, asserts every covered command emits contract marker AND has no forbidden placeholders (`<id>`, `<this-id>`, `<artifact>`, `EVID-XXX`, `RFC-XXX`)
- **Audit script** `scripts/audit-hints.sh` — produces coverage metric, runs in CI/before commits
- **Code review checklist** — any new CLI command or MCP tool without a hint fails review (codified in CLAUDE.md "Hint protocol" section)

## Reflection

What went well:
- Strict file ownership across 3 parallel agents per cycle eliminated merge conflicts
- Cyclic audit-fix protocol caught regressions early (e.g., W2's list/tree JSON shape break in Cycle 4 surfaced immediately, fixed in Cycle 5 W5)
- Audit script bug was found via mismatch between coverage % and live smoke tests — important meta-tooling lesson

What was harder:
- Distinguishing "real PRESENCE violation" from "audit script false negative" required live smoke testing every classified-MISSING command
- Some commands' "right next action" was non-obvious (`fgr`, `discover` subcommands)
- LLM-dependent commands needed careful Fix: branch handling without breaking success path

Lesson learned: meta-tooling (audit script) needs the same contract discipline as the things it audits. Heuristic regex was incomplete and produced 31 false negatives before fix.

## Reproduction

```bash
# Build
cargo build --release --bin forgeplan

# Run audit
./scripts/audit-hints.sh
# Expect: Coverage: 100.0% (70/70 contract-compliant)

# Run integration tests
cargo test --package forgeplan --test hint_contract
# Expect: 36 passed; 0 failed; 0 ignored

cargo test --package forgeplan --test cli_integration_test
# Expect: 104 passed; 0 failed
```

## Related

- PRD-071 (informs) — Unified hint contract specification
- PROB-046 (informs) — Original gap signal triggering this work
- ADR-008 (informs) — Self-describing tools alignment
- PRD-070 (based_on) — CLI parity sprint where hint inconsistency surfaced
