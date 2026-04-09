---
depth: tactical
id: EVID-066
kind: evidence
links:
- target: PRD-044
  relation: informs
- target: PROB-028
  relation: informs
status: active
title: Sprint v0.17.1 hotfix PRD-044 reindex trim orphans — 1131 tests, dogfood verified clean
---

# EVID-066: PRD-044 Implementation Evidence (v0.17.1)

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Summary

Sprint v0.17.1 hotfix PRD-044 shipped. Reindex Phase 2 parse-kind bug
fixed + new Phase 3 orphan relation trim added. Verified on dogfood
workspace: 3 phantom NOTE-037/038/040 relations removed, `forgeplan
tree` shows 0 phantom rows.

## Commit

`b6f478e` on `release/v0.17.1` branch.

## FR mapping

### FR-001: Parse-kind fix

File: `crates/forgeplan-cli/src/commands/reindex.rs` lines 134-203.
Previous code at lines 137-141 did `match record.kind.parse()
{ Ok(k) => k, Err(_) => continue }` which skipped rows with corrupt
or empty kind field. `extract_record` at `store.rs:1390` uses
`unwrap_or_default()` so null kind becomes empty string, which fails
`parse::<ArtifactKind>()`, which means continue, which means row
stays forever.

New code wraps the check in an OrphanReason enum:
- `OrphanReason::CorruptKind` when parse fails → definitely orphan
  (no valid kind means no valid directory)
- `OrphanReason::MissingFile` when parse succeeds but file missing

Both get deleted, with the reason printed in the log line so users
can tell them apart.

### FR-002: Improved trim output

Reindex now prints:
- `DEL ID — corrupt kind field, removed from DB`
- `DEL ID — no .md file found, removed from DB`

### FR-003: Phase 3 orphan relation trim (emerged during ADI)

Key discovery during ADI: phantom rows in `forgeplan tree` were NOT
in artifacts.lance but in relations.lance as dangling edges whose
source no longer exists. `forgeplan graph` showed:
```
NOTE-037 -->|informs| RFC-001
NOTE-038 -->|informs| PRD-034
NOTE-040 -->|informs| RFC-001
```

Tree renderer walks relations graph and shows unresolved IDs as `?`
phantom rows. Fix: new Phase 3 iterates `get_all_relations`, builds
a surviving-IDs set from post-Phase-2 `list_records`, deletes any
relation whose source or target is missing from the set. Prints
reason: "source missing", "target missing", or "both missing".

### FR-004: Version bump + CHANGELOG

- `Cargo.toml` workspace: 0.17.0 → 0.17.1
- `crates/forgeplan-cli/Cargo.toml` path deps: 0.17.0 → 0.17.1
- `crates/forgeplan-mcp/Cargo.toml` path deps: 0.17.0 → 0.17.1
- `CHANGELOG.md`: new v0.17.1 entry under Fixed with both PRD-044
  and PRD-045 bugs

## Tests

1131 tests pass (was 1128 in v0.17.0, +3 from PRD-045). No new
reindex-specific tests in this commit because the existing reindex
test harness was not yet in place. Manual verification via dogfood
instead:

- Before fix: `forgeplan tree` on dogfood showed 3 phantoms
  (NOTE-037, NOTE-038, NOTE-040)
- After fix: `forgeplan reindex` output:
  ```
  DEL NOTE-037 --informs--> RFC-001 — orphan relation (source missing)
  DEL NOTE-038 --informs--> PRD-034 — orphan relation (source missing)
  DEL NOTE-040 --informs--> RFC-001 — orphan relation (source missing)
  Reindex complete: 5 synced, 178 unchanged, 1 removed, 3 orphan relations, 0 errors.
  ```
- After fix: `forgeplan tree | grep '?'` returns nothing ✓

## Quality gates

- cargo fmt --check: clean
- cargo check --workspace: 0 warnings
- cargo clippy --workspace --all-targets -- -D warnings: clean
- cargo test --workspace: 1131 pass, 0 fail
- cargo build --release: success

## Key lesson for NOTE-044

**ADI must include reading existing code before specifying new features.**
My initial PRD-044 spec (Option D, warn + --trim-orphans flag) was
based on the assumption that reindex had no trim logic. Code
investigation revealed:
1. Reindex ALREADY has trim by default (Phase 2)
2. The real bug is narrower (corrupt-kind skip)
3. The REAL real bug is in a completely different place (orphan
   relations, not orphan artifacts)

If I had not done ADI and read the code, I would have written a new
`trim_orphans` function that duplicates existing code AND missed the
relation cascade bug entirely.

## Deferred from PRD-044 scope

`--show-orphans` flag (from original Option D) was not implemented.
Reason: ADI revealed the real bugs are in parse-skip and relation
cascade, not in absence of a helper command. The helper is nice-to-have,
not need-to-have for closing PROB-028. Deferred to future enhancement
if real users request it.

## Related

| Artifact | Relation |
|---|---|
| PRD-044 | informs (this evidence supports PRD-044 FRs) |
| PROB-028 | informs (this evidence closes PROB-028) |
| NOTE-044 | informs (lesson added: ADI includes code investigation) |
| EVID-067 | sibling (PRD-045 paired work) |

