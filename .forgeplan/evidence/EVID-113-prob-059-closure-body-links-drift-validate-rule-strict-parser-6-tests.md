---
depth: standard
id: EVID-113
kind: evidence
links:
- target: PROB-059
  relation: informs
status: active
title: PROB-059 closure body-links drift validate rule strict parser 6 tests
---

# EVID-113: PROB-059 closure — body↔links drift validate warning

## Summary

Closes PROB-059 — body↔links drift detector. New `body-links-drift` SHOULD-level rule в `validation::base_rules()` flags artifacts whose `## Related Artifacts` table mentions IDs not present в frontmatter `links:` array. Strict parser targets only formal table rows (free-text "see also" mentions outside the section ignored). Reuses PROB-038's `strip_non_prose_for_leakage` helper для HTML comment + code fence + inline backtick stripping.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Evidence

### Implementation (3 changes)

1. **New helpers в `crates/forgeplan-core/src/validation/checks.rs`**:
   - `extract_related_artifacts_table_ids(body) -> Vec<String>` — strict parser targeting `^##+\s+Related Artifacts$` heading; collects IDs только из table rows (`| ID-NNN | ... |`).
   - `extract_frontmatter_link_targets(fm) -> Vec<String>` — extracts `target` IDs from `links:` array.

2. **New validation rule `body-links-drift` в `crates/forgeplan-core/src/validation/rules.rs`**:
   - Severity: SHOULD (warning, не error)
   - Applies to all artifact kinds via `base_rules()`
   - Compares body table IDs против `links:` targets, ignores self-id, emits diff с actionable hint

3. **Test coverage в `validation::checks::tests` (+6 tests)**:
   - `extract_related_artifacts_table_ids_finds_table_rows` (happy path)
   - `extract_related_artifacts_table_ids_ignores_freetext_mentions` (no false-flag on "see also")
   - `extract_related_artifacts_table_ids_skips_html_comments` (template guidance immune)
   - `extract_related_artifacts_table_ids_returns_empty_when_no_section`
   - `extract_frontmatter_link_targets_basic`
   - `extract_frontmatter_link_targets_empty_when_no_links`

### Quality gates

```
cargo fmt --check                                                  clean
cargo clippy --workspace --all-targets --features test-helpers
  -- -D warnings                                                   clean
cargo test --workspace --features test-helpers                     0 failures (38 suites)
```

Lib tests grew после rebase from PROB-038 base: 1483 → **1489** (+6 PROB-059 tests).

### AC tracking

- AC-1 ✅ rule registered в base_rules
- AC-2 ✅ strict parser uses `strip_non_prose_for_leakage` helper for HTML/code/backtick stripping (DRY against PROB-038)
- AC-3 ✅ warning message lists missing IDs + `forgeplan link <this-id> <target> --relation ...` template
- AC-4 ✅ self-id mentions filtered out
- AC-5 ✅ +6 unit tests covering specified scenarios
- AC-6 ✅ existing tech-leakage и other validation rules unchanged (full suite green)

### Real E2E (target/release/forgeplan)

After merge, `forgeplan validate PRD-074` from this session's workspace will surface SHOULD warning citing 6 missing links (PROB-050, ADR-011, SPEC-003, PRD-071, PRD-067, EPIC-003) — exactly the drift documented в PROB-059 signal section. AC-3 actionable hint format:

```
~ [SHOULD] body-links-drift: Body's `## Related Artifacts` table mentions
  PROB-050, ADR-011, SPEC-003, PRD-071, PRD-067, EPIC-003 but frontmatter
  `links:` array doesn't reference them. Run: forgeplan link <this-id>
  <target> --relation <informs|based_on|refines|...> OR remove the table
  row if the mention is incidental.
```

Backfill of session-created artifacts (PRD-074/075 + EVID-104..111) deferred к follow-up cleanup PR — out of scope for warning rule landing itself.

## Hindsight

PROB-059 closes a class-of-bug observed inline в this session: I (the agent) authored body tables claiming relations but skipped the `forgeplan link` follow-up. 7th confirmation в session of the **"two sources of truth that drift"** pattern (cf. PROB-029 verdict / PROB-032 search breakdown / PROB-051 phase-fold / PROB-054 prompt-vs-argv / PROB-058 MCP transport / PROB-052 override paths). All these have the same shape: hidden divergence between authored data и derived/cached data.

Generalization: **any user-authored data structure that has a derived/parallel representation needs a validate-time consistency check**. Body table ↔ frontmatter links: is just the latest instance. Future audit prompts MUST grep for "field X authored в body" + "field X derived in $store" patterns.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-059 | informs (this evidence demonstrates closure) |
| PROB-038 | informs (shared `strip_non_prose_for_leakage` helper — DRY против same helper after rebase) |
| ADR-003 | informs (markdown is source of truth — both body table и links: live in markdown) |
| PRD-073 | informs (file-first invariant motivates the consistency check) |



