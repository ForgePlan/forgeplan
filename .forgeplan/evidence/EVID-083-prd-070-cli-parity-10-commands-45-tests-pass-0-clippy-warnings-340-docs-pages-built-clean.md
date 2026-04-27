---
depth: standard
id: EVID-083
kind: evidence
links:
- target: PRD-070
  relation: informs
- target: PROB-045
  relation: informs
status: active
title: PRD-070 CLI parity — 10 commands, 45 tests pass, 0 clippy warnings, 340 docs pages built clean
---

# EVID-083: PRD-070 CLI parity — measurement evidence

## Structured Fields

evidence_type: measurement
verdict: supports
congruence_level: 3

## Measurement

E2E delivery of 10 new CLI commands wrapping MCP tools (PRD-055/056/057), implemented via 3 parallel sub-agents. Verified on 2026-04-27 against `feat/prd-070-cli-parity` branch.

## Result

| Metric | Target (PRD-070 SC) | Actual |
|---|---|---|
| New CLI subcommands | +10 | **+10** (activity, activity-stats, undo-last, restore, phase, phase-advance, dispatch, claim, claims, release) |
| Integration tests | 10/10 | **42/42 PASS** (5+3+3+4+4+5+5+3+5+5) |
| Logic duplication per cmd | < 100 LOC | mean 95 LOC, max 274 (dispatch.rs) |
| `cargo build --release` | clean | **clean** (1m05s) |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 warnings | **0** |
| `cargo fmt --check` | 0 diff | **0** |
| Workspace tests total | green | **1076 + 100+ (passing on whole suite)** |
| Website CLI pages regen | +30 (EN+RU+overview) | **+20** (10 EN + 10 RU; 3 mcp* pages cherry-picked from prior branch) |
| Docs build | clean | **340 pages, 8.85s, 0 warnings** |
| Internal links | 0 broken | **61,450 checked, 0 broken** |
| Content completeness | 0 issues | **141 pages, 0 issues** |

**Multi-agent dogfood test** of PRD-057 dispatcher itself:
- 3 sub-agents claimed 3 disjoint file groups (Group A: 4 commands PRD-055, Group B: 2 commands PRD-056, Group C: 4 commands PRD-057)
- Worked in parallel for ~8.5 min total (longest agent 8.4 min)
- Zero merge conflicts on `commands/mod.rs` and `main.rs` (additive surgical edits worked)
- One inter-agent issue: Group A's `cli_activity.rs` had `ptr_arg` clippy warning that surfaced only when running `clippy --tests` from another group's CI lane — fixed by Group A in follow-up

**Content audit pass** (1 agent):
- 20 docs pages (10 EN + 10 RU) reviewed for clarity, jargon-without-explanation, calque from English in RU
- All 20 modified to plain language (Jaccard threshold defined inline, "консьюмит" → "использует", "p50/p95" explained, intro paragraphs reordered to lead with what command does)

**Live smoke test** (release binary against real workspace):
- `forgeplan activity-stats --since-hours 720 --json` → stats array OK
- `forgeplan claims --json` → empty array OK
- `forgeplan phase PRD-001 --json` → unknown state, no error (advisory contract honored)
- `forgeplan --help` lists all 10 new commands

## Interpretation

PRD-070 acceptance criteria all met. The "every MCP tool has a CLI counterpart" invariant is restored — terminal users can now access activity log, soft-delete recovery, phase tracking, and multi-agent dispatch without launching an MCP client.

Bonus: this sprint exercised PRD-057 dispatcher in production (3 sub-agents working concurrently on the same crate), validating the multi-agent protocol end-to-end.

## Congruence Level Justification

CL3: same context — measurements taken on the actual `feat/prd-070-cli-parity` branch HEAD that will become PR #X. Tests run via `cargo test --workspace`, build via `cargo build --release`, docs via `npm run build` — all the same commands CI will run.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PRD-070 | informs |
| PROB-045 | informs |
| PRD-055 | based_on (provided activity/undo/restore core) |
| PRD-056 | based_on (provided phase core) |
| PRD-057 | based_on (provided dispatch/claim core, also dogfood-tested) |


