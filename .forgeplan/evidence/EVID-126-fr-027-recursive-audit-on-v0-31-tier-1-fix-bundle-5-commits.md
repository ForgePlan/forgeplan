---
depth: standard
id: EVID-126
kind: evidence
last_modified_at: 2026-05-14T10:49:30.131033+00:00
last_modified_by: claude-code/2.1.141
links:
- target: PRD-077
  relation: informs
status: active
title: FR-027 — recursive audit on v0.31 TIER 1 fix bundle (5 commits)
---

# EVID-126: FR-027 — Recursive audit on v0.31 TIER 1 fix bundle

## Structured Fields

verdict: weakens
congruence_level: 3
evidence_type: code_review

## Scope

PRD-077 FR-027 — audit the 5 commits that closed v0.31.0 TIER 1 findings to verify the fixes themselves did not introduce regressions or rely on un-enforced invariants.

Commits under review:
| SHA | Subject | Scope |
|---|---|---|
| `b5a21bf` | 11 inline fixes (CR-001/002, SEC-001/002, LOG-001, ARCH-001/002, API-001, TST-001, DOC-001/002, D-LOW-2) | mixed |
| `cef1695` | validate_title lift + MCP error sanitisation (SEC-C1+C2+H3) | input gate |
| `36c720d` | `health_report_to_json` helper extract (ARCH-C1) | architectural |
| `897377b` | bare HOME mid-string masking (SEC-H2) | sanitiser |
| `095bc4b` | `sanitize_for_hint` in 8 CLI commands (SEC-H1) | output gate |

## Method

Per-commit diff read via `git show <sha>`, plus targeted greps for the invariants each commit claimed to establish. Cross-checked against round-1 and round-2 Wave 9 audit reports + round-1 Wave 1 audit on the v0.32 integration trial.

## Findings

### 1. Pre-existing findings already triaged (re-confirming, NOT new)

These four issues were already caught by round-1 / round-2 audits during the v0.32 sprint and are addressed in the current integration trial. Re-documenting here to close the FR-027 loop, not as new findings.

| Origin commit | Finding | Status |
|---|---|---|
| `cef1695` | `sanitize_error_chain` masks paths but NOT newlines / NUL / shell metas — a future `with_context("loaded from {path}")` upstream of `require_llm_config` would forge a second `Fix:` line | Mitigated via sanitize_for_hint wraps on every interpolation; full closure pending E1 below |
| `095bc4b` | 3 of 11 `secrets.yaml` strings missed in reason.rs lines 50/195/213/518 — file renamed to `secrets.env` but original sweep was incomplete | Closed in audit-r2 commit `150ed3e` |
| `b5a21bf` | Verdict aggregator at-risk threshold tests were missing the triplet that the audit added in CR-002 | Closed inline at land time |
| `36c720d` | `health_report_to_json` helper extracted in `b5a21bf`/`36c720d`; W4's `phase_read_errors` addition could have desynced CLI vs MCP wire shape | Closed by helper architecture — both surfaces went through the helper, so adding a field landed in both atomically (verified post-merge) |

### 2. New finding (FR-027 mandate — ≥1 required)

**E1 (NEW, MED) — No compile-time or test-time invariant for "one `Fix:` line per stderr" contract.**

- **Origin**: implicit invariant established in `b5a21bf` + `cef1695`, documented in PRD-071 hint protocol (`CLAUDE.md`), enforced only via code review.
- **Why this matters**: when round-2 audit caught the double `Fix:` regression in `reason.rs`, the contract had been silently broken by the W2 worker who added a second `eprintln!("Fix: ...")` after the anyhow error already carried one. Tests passed. Clippy passed. Only manual audit caught it. The same regression can re-enter at any of the 47 `Fix:`-emitting sites across CLI + core if an unaware contributor adds a sibling line.
- **Reproduction (trace)**:
  - `grep -rn '^\s*eprintln!.*Fix:' crates/` → multiple call sites, no static analysis
  - `grep -rn 'anyhow::bail!.*Fix:' crates/` → another set, can co-occur with above
  - No test asserts `Fix: line count == 1` for the missing-LLM / missing-key / bad-config error paths
- **Suggested fix** (not implemented as part of this audit — scope is to surface, not patch):
  - Integration test that exercises ALL error paths in `reason.rs::run` with adversarial config and asserts `stderr.lines().filter(|l| l.starts_with("Fix:")).count() == 1`
  - OR a lint via `cargo clippy` custom rule + CI check
  - OR a `Hint` builder type in `forgeplan-core::hints` that the call site builds and that the `Display` impl renders exactly once
- **Severity rationale**: MED. Not a CVE-class issue — but a regression that already happened once is likely to happen again, and the round-2 reviewer was lucky to catch it manually.

### 3. Cross-fix interaction check — clean

The five commits land in different layers (sanitiser, error chain, helper, validate, CLI prints). The only cross-fix interaction surface is `health_report_to_json` extracted in `36c720d`:
- CLI consumer ✅ uses helper
- MCP consumer ✅ uses helper (via `health_report_with_phase` returning `HealthReport`)
- W4's `phase_read_errors` addition (separate commit, post-`36c720d`) was correctly propagated by helper to both surfaces — verified by reading `commands/health.rs` + `mcp/src/server.rs` against helper signature

No silent breakage between fixes.

## Acceptance Criteria for FR-027

- [x] All 5 commits read in full
- [x] ≥1 new finding (E1)
- [x] Cross-fix interactions checked
- [x] EVID written with structured fields
- [x] Linked to PRD-077

## Recommendation

Land E1 as a follow-up PROB for v0.33.0. It's not a v0.32 release-blocker — the current code is correct, but the lack of regression-fence makes it easy to re-break. Filing a PROB keeps it tracked without expanding v0.32 scope further.

## Related Artifacts

- PRD-077 (informs — closes FR-027 acceptance)
- EVID-122 (based_on — v0.31.0 Wave 9 closure that produced these 5 commits)
- Round-1 Wave 9 audit findings (CR-001/002, SEC-001/H1/H2/H3, etc.)
- Round-2 Wave 9 audit findings (double `Fix:`, missed `secrets.yaml` strings)
- Round-1 + Round-2 Wave 1 audit findings on v0.32 integration trial

## Reproduction commands

```bash
# Inspect the 5 commits
for sha in b5a21bf cef1695 36c720d 897377b 095bc4b; do
  git show "$sha" -- crates/
done

# Verify E1 — count Fix: emit sites across the codebase
grep -rEn '(eprintln|anyhow::bail).*Fix:' crates/ | wc -l

# Verify cross-fix interaction — helper used by both surfaces
grep -rn 'health_report_to_json' crates/
```




