---
depth: tactical
id: PROB-027
kind: problem
links:
- target: EPIC-003
  relation: informs
- target: EVID-060
  relation: informs
status: draft
title: forgeplan reindex cannot rebuild LanceDB from scratch when lance/ dir is missing
---

# PROB-027: forgeplan reindex cannot rebuild from zero

## Signal

Discovered during Sprint 13.3 W6 E2E verification.

Reproduction:
```bash
forgeplan init -y
forgeplan new prd "Test"
forgeplan tag PRD-001 source=code     # updates LanceDB + md file
rm -rf .forgeplan/lance                # simulate lost index / fresh clone scenario
forgeplan reindex                      # expected: rebuild from .md files
```

Actual output:
```
Error: Table 'artifacts' was not found
Caused by: Dataset at path .forgeplan/lance/artifacts.lance was not found
```

`forgeplan reindex` currently **requires an existing** `artifacts.lance` table — it syncs markdown changes INTO the table but cannot CREATE the table from scratch.

## Context (why this matters for ADR-003)

ADR-003 declares files as source of truth. Fresh clone workflow per CLAUDE.md:
```
git clone <repo> && cd forgeplan
forgeplan init -y        # creates empty .forgeplan/lance/
forgeplan scan-import    # rebuilds index from tracked markdown
```

So `scan-import` is the canonical "rebuild index from files" command, and `init -y` creates the empty table first. The bug is that `reindex` is documented / expected to do the same but doesn't.

## Root Cause (hypothesis)

`reindex` opens the LanceDB table with `Table::open(...)` which fails when the dataset doesn't exist, rather than `Table::create_or_open(...)`. The command assumes the table is already present.

Alternatively, `reindex` and `scan-import` should converge — currently two commands do similar things with subtly different semantics.

## Constraints

- Must not break existing `reindex` behavior for users who have a valid table
- Must not create a table in the wrong location / schema
- If bug is in `reindex` itself, don't regress `scan-import`

## Optimization Targets

1. **Robust rebuild-from-zero** — `reindex` should create the table if missing OR should fail with a clear message pointing to `scan-import`
2. **Converge reindex and scan-import semantics** — they should be the same command, or clearly differentiated in help text
3. **E2E regression test** — the reindex-from-zero scenario should be in cli_integration_test

## Acceptance Criteria

1. `forgeplan reindex` on a workspace with missing `.forgeplan/lance/` either:
   - (a) automatically creates the table and rebuilds from markdown (preferred), OR
   - (b) fails with a clear message: "LanceDB table missing — run `forgeplan scan-import` first"
2. Regression test in `cli_integration_test.rs`:
   - init → new → tag → rm lance → reindex → list --tag <tag> returns the artifact
3. Help text for `reindex` and `scan-import` clearly explains the difference

## Blast Radius

- crates/forgeplan-cli/src/commands/reindex.rs (main fix)
- crates/forgeplan-core/src/db/store.rs (maybe LanceStore::open_or_create helper)
- crates/forgeplan-cli/tests/cli_integration_test.rs (regression test)

## Reversibility

**High** — change is additive (auto-create on missing), no data migration, no API changes.

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-003 | informs (files-first architecture contract) |
| EVID-060 | informs (discovered during Sprint 13.3 W6 E2E) |
| EPIC-003 | informs |
