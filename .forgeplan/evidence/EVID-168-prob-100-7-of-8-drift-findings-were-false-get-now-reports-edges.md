---
depth: tactical
id: EVID-168
kind: evidence
links:
- target: PROB-100
  relation: informs
status: draft
title: 'PROB-100: 7 of 8 drift findings were false; get now reports edges'
---

---
assigned_number: 168
predicted_number: 168
slug: evid-prob-100-7-of-8-drift-findings-were-false-get-now-reports-edges
---

# EVID-168: measured before and after, on real artifacts

Both defects in PROB-100 were reproduced on this workspace's own artifacts, not
on fixtures, and re-measured after the fix. Fixtures would not have caught either
one: every layer worked in isolation.

## body-links-drift (#446)

`forgeplan validate SPEC-003`

| | ids named | false | true |
|---|---|---|---|
| before | 8 | 7 | 1 |
| after | 1 | 0 | 1 |

The seven false ones: `ADR-009`, `PRD-065`, `SPEC-004` are real edges —
`forgeplan graph` prints all three for `SPEC-003` — and `FR-1`, `FR-2`, `FR-3`,
`FR-5` are requirement numbers that can never be link targets.

The one true finding, `EPIC-007`, survives the fix. Checked directly: the graph
has no `SPEC-003 → EPIC-007` edge, so the body table names something the artifact
is not linked to. That is what the rule is for.

`forgeplan validate ADR-009`: 10 ids named before, 8 after. The two that dropped
(`ADR-008`, `PROB-042`) are real edges.

## forgeplan get (#447)

`forgeplan get SPEC-003 --json`

- before: 14 keys, none about links
- after: `links.outbound` = `PRD-065 (refines)`, `ADR-009 (based_on)`,
  `SPEC-004 (informs)` — identical to `forgeplan graph`
- after: `links.inbound` = `EVID-089 (informs)`, which
  `graph | grep "SPEC-003 -->"` cannot show at all

Empty case, fresh workspace: `{"outbound": [], "inbound": []}` and `Links: none`.
The field is present either way, so "no links" is distinguishable from "not
reported" — the ambiguity the issue was filed about.

## Tests

8 new. 2 on the prefix filter, 3 on the store merge, 3 end-to-end on the real
binary against a real workspace.

The prefix-filter test was mutation-checked: with the filter removed it fails.
It detects the defect rather than confirming current behaviour — the distinction
that let #348 survive for months behind a test asserting the broken string.

## Gate results

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0, 0 diffs |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0, 0 warnings |
| `cargo test --workspace --no-fail-fast` | 3295 passed, 3 failed, 92 binaries |

The 3 failures are #454 / PROB-090, not this change. Each passes in isolation and
fails only under parallel load; none touches the modified code. Two are in
`git/`, untouched here. The third, `c33_forgeplan_decompose_no_llm_smoke`,
asserts that **no** LLM provider is configured — it breaks when a sibling test
sets the variable. That widens #454, whose title says "flaky git tests": the
class is any test asserting on process-global state. CI does not see it because
it runs `cargo nextest`, one process per test; local `cargo test` shares one.

Disk was at 100% with 6.2 GiB free before this run. That state previously
produced `passed=0 failed=0` at exit 0 — a gate reporting a value that is not a
result. 37 GiB was freed before measuring, so these numbers are from a run that
actually happened.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: measurement

base_sha: ca5a7c2c
result_sha: 9dd7242
changed_paths: crates/forgeplan-cli/src/commands/get.rs, crates/forgeplan-cli/src/commands/validate.rs, crates/forgeplan-cli/tests/cli_get_links.rs, crates/forgeplan-core/src/db/store.rs, crates/forgeplan-core/src/lifecycle/mod.rs, crates/forgeplan-core/src/validation/checks.rs, crates/forgeplan-core/src/validation/rules.rs, crates/forgeplan-mcp/src/convert.rs, crates/forgeplan-mcp/src/server.rs, crates/forgeplan-mcp/src/types.rs, .gitignore


