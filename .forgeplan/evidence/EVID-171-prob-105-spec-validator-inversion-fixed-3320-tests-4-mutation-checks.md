---
depth: tactical
id: EVID-171
kind: evidence
links:
- target: PROB-105
  relation: informs
status: active
title: 'PROB-105: SPEC validator inversion fixed — 3320 tests, 4 mutation checks'
---

---
assigned_number: 171
predicted_number: 171
slug: evid-prob-105-spec-validator-inversion-fixed-3320-tests-4-mutation-checks
---

# EVID-171: the SPEC validator inversion, measured before and after

## Summary

PROB-105 claimed the SPEC validator was inverted: it passed empty templates and
blocked real behavioural specs. Both halves reproduced on 0.36.0 with the shipped
binary, and both are closed by `daa103b` + `c8e7bd3` on `fix/spec-validator-inverted`.

## What was measured

| Input | Before | After |
|---|---|---|
| untouched `forgeplan new spec` template | `PASS — 0 error(s), 0 warning(s)`, activates at R_eff 1.00 | stub warning; `activate` refuses |
| behavioural spec, 2 requirements + 2 GIVEN/WHEN/THEN scenarios | `x [MUST] spec-contracts` → FAIL, cannot activate | PASS |
| SPEC-002 … SPEC-006 | pass | unchanged |
| SPEC-001 | fails `spec-summary` + `spec-contracts` | unchanged — a document *about* writing specs, filed as a spec |

Corpus measurements that set the threshold, rather than taste:

- placeholders: SPEC template **15**, PRD template **5**, the six real SPECs **0–3**.
  `MANY_PLACEHOLDERS = 8` sits in the gap with room on both sides.
- a line-count test was measured and rejected: the untouched template has 25
  non-empty lines under `## API Contracts`, because placeholder JSON is still lines.
- subjective-adjective hits across the 69 PRDs: **2** in FR (already checked),
  **15** in NFR (never checked). All 15 sit inside the template's own
  `<!-- BAD: "System should be fast and responsive" -->` guidance, so a rule
  flagging them would be unclosable. With the non-prose strip: **0 findings across
  all 69 PRDs**, while hand-written vague prose still produces findings with line
  numbers.
- the blanket #450 rule was measured before being rejected: fires on 6 of 6 SPECs
  here, none defective. The shipped rule is conditional.

## Test run

Per crate — a full `cargo test --workspace` does not fit in this machine's free
space (`ld: write() failed, errno=28` twice at ~300 MB free).

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, 0 warnings |
| `cargo test -p forgeplan-core --features test-helpers --no-fail-fast` | 2239 passed, 4 failed (17 binaries) |
| `cargo test -p forgeplan --no-fail-fast` | 807 passed, 0 failed (58 binaries) |
| `cargo test -p forgeplan-mcp --no-fail-fast` | 274 passed, 0 failed (19 binaries) |

**3320 passed, 4 failed, 94 test binaries.** The binary count matches what
`--workspace` produces, so the per-crate split skipped nothing. The 4 failures are
all `git::tests` (#454): they pass serially (51/51 under `--test-threads=1`) and
this diff does not touch that module.

## Mutation checks

Each fix reverted; the matching test had to fail. A test that passes on broken
code proves nothing, so each was checked rather than assumed.

| Mutation | Test | Result |
|---|---|---|
| remove the HTML-comment strip | `nfr_adjectives_inside_html_comments_are_not_flagged` | FAILED |
| remove the placeholder scaling | untouched template back to `PASS — 0 error(s), 0 warning(s)` | reproduced |
| `MANY_PLACEHOLDERS = 100` | `the_shipped_spec_template_is_a_stub_and_declares_no_sections` | FAILED |
| append a real `## Requirements` heading to the template | same test, second assertion | FAILED |

Four pre-existing rule-count tests failed and were updated, not silenced. They sum
named groups with comments rather than asserting a bare number, so updating one
means explaining where the new term came from. `rules_for_spec_returns_base_plus_3`
was renamed to `_plus_4` rather than edited in place — a test name carrying a number
goes stale silently.

## Limits of this pack

- It certifies the validator's behaviour on this repository's corpus (69 PRDs, 6
  SPECs) and on the shipped template. It says nothing about corpora with other
  spec conventions.
- `check_measurability_adjectives` (the FR-side rule) still lacks the non-prose
  strip and carries the same latent bug. It has never fired only because the
  template's BAD examples happen to live under the NFR heading. Out of scope here,
  and recorded so it is not rediscovered.
- The 4 `git::tests` failures are a known pre-existing flake (#454), not evidence
  of health in that module.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test
base_sha: 5e9b9a9f
result_sha: c8e7bd3
changed_paths: crates/forgeplan-core/src/validation/rules.rs, crates/forgeplan-core/src/validation/checks.rs, crates/forgeplan-core/src/lifecycle/mod.rs, templates/spec/_TEMPLATE.md, CHANGELOG.md, docs/handoff/spec-validator-and-nfr-rules.md



