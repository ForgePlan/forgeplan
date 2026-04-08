---
depth: tactical
id: EVID-063
kind: evidence
links:
- target: PRD-041
  relation: informs
status: active
title: Sprint 13.6 PRD-041 FPF Rules CLI + MCP — 1075 tests pass, 12/12 E2E, READY TO MERGE
---

# EVID-063: Sprint 13.6 PRD-041 Implementation Evidence

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Summary

Sprint 13.6 implemented PRD-041 in full: 4 FRs (CLI `fpf rules`, CLI `fpf check`, MCP `forgeplan_fpf_rules`, MCP `forgeplan_fpf_check`) shipped on branch `feat/sprint-13.6-prd-041-fpf-rules` over `release/v0.17.0`. Multi-agent team execution with full /forge-cycle discipline: 2 parallel implementers, 1 tester, 4 parallel auditors, 1 fixer, 1 re-auditor + manual UX polish by team-lead.

## Commits (6)

| Commit | Scope | LOC |
|---|---|---|
| `7056e14` | core helpers: RuleSource, active_rules, RuleCheckResult, check_artifact_against_rules | +218 |
| `ff6babc` | MCP forgeplan_fpf_rules + forgeplan_fpf_check tools | +181 |
| `e819596` | CLI run_rules (tree/flat/json) + run_check (styled/verbose/json) + enum wiring | +436 |
| `24369aa` | unit + CLI integration + E2E regression tests | +392 |
| `76d5e7a` | audit fixes: dedupe enrichment, canonical JSON, Option refactor, Condition::summarize in core, param bounds, test strengthening | +541/-366 |
| `77b0c12` | manual UX polish: pluralization, link_count== → =, error dedup via exit(1), condition_summary in CLI JSON | +8/-5 |

## FR mapping

### FR-001 — `forgeplan fpf rules [--flat] [--json]`
Action-grouped tree by default (EXPLORE/INVESTIGATE/EXPLOIT), `--flat` = priority-linear table, `--json` = dump with condition + condition_summary per rule.
- Impl: `crates/forgeplan-cli/src/commands/fpf.rs::run_rules()`
- Tests: cli_fpf_rules_shows_default_source, cli_fpf_rules_json_valid, cli_fpf_rules_flat_has_priorities + 8 unit tests for summarize_condition/style_action/truncate

### FR-002 — `forgeplan fpf check <id> [--verbose] [--json]`
Styled: header + `★ winning` + action + message + "N other rule(s) did not match." Verbose adds unmatched list. JSON emits canonical `{artifact_id, kind, status, matched, unmatched, winning, summary}`. Missing id: clean `✗ not found → hint`, exit 1.
- Impl: `crates/forgeplan-cli/src/commands/fpf.rs::run_check()`
- Tests: cli_fpf_check_missing_artifact_errors, cli_fpf_check_existing_artifact, cli_fpf_check_verbose_shows_unmatched, cli_fpf_check_json_has_required_fields

### FR-003 — MCP `forgeplan_fpf_rules`
4 optional params: action (≤64), name (≤128), summary (bool), source (≤16). Default full dump with condition tree + condition_summary.
- Impl: `crates/forgeplan-mcp/src/server.rs::forgeplan_fpf_rules()`
- Tests: fpf_param_validation_tests (length bounds) + core active_rules tests

### FR-004 — MCP `forgeplan_fpf_check`
Single required param `id` (≤128). Canonical JSON shape (identical to CLI --json).
- Impl: `crates/forgeplan-mcp/src/server.rs::forgeplan_fpf_check()`
- Tests: param validation + core check_artifact_against_rules tests (None on missing, custom config, priority order, Serialize, summary_line)

## Core API

- `RuleSource` — Config | Default, `#[serde(rename_all="snake_case")]`
- `active_rules(fpf_config) -> (Vec<Rule>, RuleSource)`
- `RuleCheckResult` — Serialize derived, `#[serde(rename)]` for canonical kind/status
- `MatchedRule { name, priority, action, message }` — Serialize
- `RuleCheckResult::summary_line() -> String` — "N matched, N unmatched, winning: name"
- `check_artifact_against_rules(store, id, config) -> Result<Option<RuleCheckResult>>` — None on missing, Err only on real errors
- `Condition::summarize() -> String` + `CONDITION_SUMMARY_MAX: usize = 120` in `fpf/ext/rules.rs` — used by both CLI and MCP
- Internal: `build_lookup_maps` + `enrich_one` — shared O(N+R) helper between `build_rule_actions` and `check_artifact_against_rules`

## Audit cycle

### Round 1: 4 parallel auditors
| Auditor | Crit | High | Med | Low |
|---|---|---|---|---|
| Rust | 0 | 0 | 2 | 5 |
| Security | 0 | 0 | 3 | 6 |
| Architecture | 0 | 2 | 4 | 4 |
| Tests | 0 | 1-cluster + 4 sub | 7 | 5 |

**HIGH findings:**
- Arch H1: enrich_record_for_rules duplicated build_rule_actions logic + lost O(N+R) optimization → O(N·R) regression
- Arch H2: RuleCheckResult/MatchedRule without Serialize; CLI and MCP JSON already drifted (artifact_kind vs kind, summary in MCP only)
- Tests T-H1: MCP tools zero handler test coverage
- Tests sub: weak tautological assertions in 2 CLI integration tests, missing verbose/Config-source coverage

### Round 2: fixer (commit 76d5e7a) applied 7 fixes
1. Dedupe enrichment — shared `build_lookup_maps` + `enrich_one`, both paths O(N+R)
2. Canonical JSON — derive Serialize + rename, summary_line() method, retire hand-rolled JSON
3. Condition::summarize() moved to core, MCP now emits condition_summary alongside nested condition
4. MCP param length bounds + 4 unit tests
5. Result<Option<RuleCheckResult>> — kills brittle string-match, removes Lance internals leak in CLI errors
6. MCP handler harness DEFERRED (no tests/ dir infra; handlers 15 LOC thin wrappers; core well-tested; param validation at unit level)
7. Test strengthening — concrete assertions, new verbose test, 6 new core prd041_tests

### Round 3: re-auditor verification
- 6 HIGH verified PASS against actual code
- 1 PARTIAL (MCP harness) confirmed as intentional deferral
- 0 new issues introduced
- Quality gates green
- **Verdict: READY TO MERGE**

### Round 4: manual UX polish (commit 77b0c12)
Team-lead ran 7 commands on release binary. 4 cosmetic fixes:
1. "1 rules" → "1 rule" pluralization
2. `link_count==0` → `link_count=0` (format_numeric Eq variant)
3. Error dedup: `std::process::exit(1)` after ui::error_hint instead of bubbling anyhow
4. CLI fpf rules --json missing condition_summary → added for MCP parity

## Test results

- **Total: 1075 tests pass, 0 failed**, 1 ignored (baseline ~1060 + 15 net)
- New tests by location:
  - `forgeplan-core/src/fpf/mod.rs::prd041_tests`: 12 tests
  - `forgeplan-core/src/fpf/ext/rules.rs::tests`: 5 tests for Condition::summarize (migrated + extended)
  - `forgeplan-cli/src/commands/fpf.rs`: 8 unit tests
  - `forgeplan-cli/tests/fpf_rules_check.rs`: 6 integration tests (assert_cmd)
  - `forgeplan-mcp/src/server.rs::fpf_param_validation_tests`: 4 tests
- `cargo fmt --check`: clean
- `cargo check --workspace`: 0 warnings, 0 errors
- `cargo build --release`: 1m 56s, 42MB binary
- E2E regression `tests/e2e/sprint-13.6-regression.sh`: exit 0, 12/12 checks pass (Sprint 13.1 dup guard → 13.2 search → 13.3 tags → 13.4 discover → 13.5 score → 13.6 fpf rules/check)

## UX verification (manual on release binary)

| Command | Quality | Key finding |
|---|---|---|
| `fpf rules` (tree) | ★★★★★ | Action-grouped box-drawing, correct pluralization, condition summaries |
| `fpf rules --flat` | ★★★★★ | Aligned ASCII table |
| `fpf rules --json` | ★★★★★ | Canonical shape + condition_summary parity with MCP |
| `fpf check <id>` styled | ★★★★★ | ★ winning + action + message + summary |
| `fpf check --verbose` | ★★★★★ | Unmatched rules section |
| `fpf check --json` | ★★★★★ | Canonical kind/status/summary |
| `fpf check NOT-EXIST` | ★★★★★ | ✗ clean error + hint + exit 1, no duplication |

## Deferred to backlog

- **Arch M1**: move PRD-041 code from `fpf/mod.rs` to `fpf/ext/rules.rs` for organization
- **Arch M3 / Sec M1**: O(N) workspace scan per check_artifact_against_rules call — doc warning added; actual batching/caching deferred until MCP does batch-check
- **Arch M4**: `RuleQuery` helper type if third surface appears (TUI, web)
- **Tests T-H1 residual**: full MCP handler integration harness (`crates/forgeplan-mcp/tests/`) — needs design decision on handler visibility
- **Tests Medium**: unicode boundary truncate, priority tie-break, MCP filter combos
- **Rust nits**: cosmetic items from audit

## Integration with prior Sprint 13.x

- Sprint 13.2 BM25 search: regression verified
- Sprint 13.3 tags: regression verified
- Sprint 13.4 discover: regression verified
- Sprint 13.5 score with CI: regression verified
- Sprint 12 FPF Rule Engine: this sprint is the missing surface — was hidden before, now introspectable via 4 entry points

## Team execution pattern (for future sprints)

Hybrid team-up pattern worked well:
- team-lead (main) spawned agents (subagents can't use Agent tool)
- 2 parallel implementers (CLI + MCP, independent files) → both done
- 1 tester picked up unblocked W3
- 4 parallel auditors (Rust/Sec/Arch/Tests) read-only
- 1 sequential fixer (touches multiple crates — no file-conflict risk at cost of serial)
- 1 re-auditor verified
- team-lead did manual UX polish directly (faster for cosmetic)
- Total wall time: ~30 min parallel phase + ~25 min serial fix + ~10 min polish = ~65 min

## Related Artifacts

| Artifact | Relation |
|---|---|
| PRD-041 | informs (this evidence supports FR-001..FR-004) |
| EPIC-003 | informs (Sprint 13 v0.17.0 series) |
| RFC-001 | informs (Phase 3 — rules surface) |
| EVID-062 | predecessor (Sprint 13.5) |
| PRD-040 | context (Sprint 13.5) |
| PRD-042 | blocked-until-now (Sprint 13.7 needs 13.6 merged — shared fpf.rs + server.rs files) |

