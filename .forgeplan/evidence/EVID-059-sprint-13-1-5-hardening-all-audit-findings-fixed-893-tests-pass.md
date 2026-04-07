---
depth: tactical
id: EVID-059
kind: evidence
links:
- target: PRD-043
  relation: informs
status: draft
title: Sprint 13.1.5 hardening — all audit findings fixed, 893 tests pass
---

---
id: EVID-059
title: "Sprint 13.1.5 hardening — all audit findings fixed, 893 tests pass"
status: Draft
created: 2026-04-07
---

# EVID-059: Sprint 13.1.5 Hardening Evidence

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Summary

Sprint 13.1.5 applied 7 fixes addressing all HIGH findings from the Sprint 13.1 multi-agent audit (1C + 6H across 4 specialized auditors: Rust, Security, Architecture, Test Coverage). Plus added config-driven MCP size limits per user request.

## Fixes applied

### F1 — LazyLock for regex in check_stub
- File: `crates/forgeplan-core/src/validation/rules.rs`
- `PLACEHOLDER_RE: LazyLock<Regex>` static — compiled once, reused forever
- Eliminates regex recompilation on every validation call
- Audit: Rust H-1, Security M-1

### F2 — StubReport typed return
- New: `pub struct StubReport { count, message }`
- New: `pub fn check_stub_detailed(body, fm) -> Option<StubReport>`
- `check_stub` becomes thin wrapper calling detailed variant
- `health::find_active_stubs` no longer parses count from message text
- Audit: Rust H-3, Architecture M-3

### F3 — Import gate for active stubs (security fix)
- File: `crates/forgeplan-cli/src/commands/import_cmd.rs`
- Active records in import JSON run through check_stub_detailed
- If stub detected → warning + downgrade to draft (unless --force)
- Closes the ONLY unintentional bypass found by security audit
- Audit: Security M-3

### F4 — IntegrityConfig (configurable thresholds + MCP limits)
- File: `crates/forgeplan-core/src/config/types.rs`
- 5 fields: duplicate_threshold, duplicate_pairs_limit, stub_marker_threshold, mcp_max_title_len (256), mcp_max_body_len (1 MB)
- Per-field serde default — partial YAML overrides work
- MCP forgeplan_new + forgeplan_update validate sizes
- Audit: Security L-5 + user request

### F5 — collect_activation_gates DRY helper
- File: `crates/forgeplan-core/src/lifecycle/mod.rs`
- New GatesReport struct + async fn collect_activation_gates
- review() and activate() both use it → consistent verdicts
- Wires check_stub after rebase (replaces length proxy)
- Audit: Architecture M-4

### F6 — --force backward-compat alias
- File: `crates/forgeplan-cli/src/main.rs`
- `#[arg(long, visible_alias = "force")]` on allow_duplicate
- Old scripts using --force still work
- Audit: Architecture L-1

### F7 — 6 missing tests from coverage audit
- Boundary test (Jaccard exactly 0.7 fires — catches `>=` → `>` regression)
- --allow-duplicate E2E CLI integration test
- MCP forgeplan_new handler returns warnings field
- Each English marker individually catches stub
- find_active_stubs end-to-end (scan-import bypass detection)
- forgeplan import rejects active stub test
- Audit: Test Coverage H-1..H-3 + gaps

### BONUS — README marketplace link
- README.md + README.ru.md: marketplace/ → https://github.com/ForgePlan/marketplace

## Test results

- Total: 893 tests pass, 0 fail (up from 879)
- New tests: ~14
- cargo fmt --check: clean
- cargo check --workspace: 0 warnings
- cargo build --workspace: clean
- 0 new dependencies

## Sprint methodology

- Branch: feat/sprint-13.1.5-hardening
- Hybrid pattern (NOTE-043): main thread spawn + team-lead coordinate
- 6 fixers + 1 lead across 3 waves
- Mid-sprint pivot: branch rebase when base-branch dependency discovered
- 1 agent gracefully shutdown when task invalidated, re-spawned on correct base

## Architectural wins

- Typed StubReport — no more stringly-typed coupling
- review()/activate() agree on gates — single source of truth
- Config-driven integrity thresholds (security + UX)
- Backward-compat preserved for --force
- Import path hardened — closes security bypass

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| PRD-043 | informs (extends) |
| PROB-024 | informs (root problem) |
| EPIC-003 | informs (Sprint 13 series) |
| NOTE-043 | informs (orchestration pattern used) |
| EVID-058 | informs (prior sprint) |
