# PROB-060 Phase 0b — Stress-test fixture (Variant B / local simulation)

This fixture drives `tests/prob_060_stress_test.rs` — the **EVID-A** evidence
gate per ADR-012's outcome-based reversal condition.

## Scenario

- **Base** (`origin/dev` simulation): a single legacy artifact `PRD-073` already
  has `assigned_number: 73` — establishes the baseline `max(assigned_number)`
  the binary must read from the base ref.
- **PR branches** (10 of them): each carries one fresh artifact with
  `assigned_number: null`. The slugs are intentionally non-overlapping so we
  measure the assignment *number-minting* logic, not slug collision behavior
  (collision behavior has dedicated unit tests in `commands/ci_assign_id.rs`).

## Why local simulation (Variant B), not real GH Actions (Variant A)

- The unit-of-test for this binary is the *function* `ci_assign_id::run`, not
  the workflow YAML. Workflow concurrency is GitHub's responsibility; serializing
  parallel merges via `concurrency: forgeplan-id-assign` is documented behavior
  we trust at the API level (CL2 framing per Worker 1 prompt).
- The test models the post-serialization world: 10 branches merged into `dev`
  one at a time in a seeded permutation. The current harness uses 3 git-backed
  seeds in the default suite and 12 in the ignored full variant; the separate
  in-process property loop covers 100 permutations. Each checks the final
  `assigned_number` set `{74, 75, …, 83}` for its tested orders.
- "Real-runtime concurrency under multi-runner contention" — Variant A — is
  Worker 2's runbook scope in the original plan. This fixture does not establish
  that Variant A ran or that either acceptance gate passed.

## Layout

```
prob_060_stress/
├── README.md                          ← this file
├── base/
│   └── .forgeplan/prds/PRD-073-existing.md   ← legacy: assigned_number: 73
└── pr_01..pr_10/
    └── .forgeplan/prds/prd-feature-NN.md     ← each: assigned_number: null
```

The fixture is plain `.md` files; the test harness clones them into a
`tempfile::TempDir`, initializes a real git repo, creates 10 branches off
`dev`, merges them in a seeded random order, and asserts on the final
frontmatter state.
