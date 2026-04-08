---
depth: tactical
id: PROB-028
kind: problem
status: active
title: Phantom rows in LanceDB persist after .md file deletion — reindex bug
---

# PROB-028: Phantom rows in LanceDB persist after .md file deletion

## Signal

`forgeplan tree` displays artifacts with `?` for kind/status/title and `0.00`
R_eff. Reproducible: NOTE-037 and NOTE-040 в `.forgeplan/dogfood` workspace
showed up as phantom rows on 2026-04-08.

```
░░░░░░░░░░  0.00  ?     ?     │     ├─ NOTE-037 "?"
░░░░░░░░░░  0.00  ?     ?     │     └─ NOTE-040 "?"
```

Verification:
- `forgeplan get NOTE-037` → "Artifact 'NOTE-037' not found"
- `ls .forgeplan/notes/NOTE-037*.md` → no match
- `grep -rln NOTE-037 .forgeplan/lance/` → 4 binary lance files contain ID
- `forgeplan reindex` ran multiple times, did NOT remove these orphan rows

## Root cause hypothesis

`reindex` walks `.forgeplan/*/*.md` files and writes them to LanceDB
(upsert). It does NOT enumerate LanceDB rows and check whether each has
a corresponding `.md` file on disk. When a user (or git pull) deletes
an `.md` file, the row in `artifacts.lance` is left orphaned.

ADR-003 explicitly says "files = source of truth, LanceDB = derived
index." The current `reindex` is one-way (files → LanceDB) and never
performs reverse trim (LanceDB rows without files → delete).

## Constraints

- ADR-003 must be respected: files are authoritative
- Reindex must remain idempotent (running it twice = same end state)
- Must not break legitimate sync from git (newly pulled files)
- Must not delete rows that have a file but the file is unreadable
  due to transient I/O issue — only delete when file is verifiably absent
- Must distinguish "file deleted" from "file moved/renamed" (id-based,
  not path-based)

## Optimization Targets

- Make `reindex` truly bidirectional: add a "trim orphans" pass
- Surface phantom row count in `health` so users notice the drift
- Zero phantom rows after `forgeplan reindex` on any clean workspace

## Observation Indicators (Anti-Goodhart)

- DO NOT optimize for "lowest LanceDB row count" — could mask failed
  inserts as "trimmed orphans"
- DO NOT optimize for "fastest reindex" if it skips the trim pass
- Track: number of rows trimmed per reindex call (should be 0 on
  steady state, > 0 only after manual file deletion)

## Acceptance Criteria

1. `reindex` deletes LanceDB rows whose `id` has no corresponding
   `.md` file in the expected directory for that kind
2. `reindex` reports trimmed count: "Reindex complete: N synced,
   M unchanged, K removed (X trimmed orphans), 0 errors"
3. After running `reindex` on the dogfood workspace, NOTE-037 and
   NOTE-040 disappear from `forgeplan tree` output
4. Idempotent: second `reindex` reports "0 trimmed orphans"
5. Test: integration test creates artifact, deletes its .md file,
   runs reindex, asserts row is removed and `forgeplan get` returns
   not-found
6. Cross-kind safety: only trim from the kind's directory (notes from
   `.forgeplan/notes/`, prds from `.forgeplan/prds/`, etc.) — never
   trim from a directory that wasn't scanned

## Blast Radius

- All forgeplan users (any workspace where files have been deleted
  externally — git pull, manual cleanup, mv operations)
- `tree`, `list`, `health`, `score`, `graph` commands all read from
  LanceDB and would silently surface phantom rows until fix lands
- LanceDB integrity: phantom rows can also break `link`/`unlink`
  graphs if the orphan ID is referenced as a relation source/target
- ADR-003 compliance: current behavior is a passive violation of
  "files = source of truth"

## Reversibility

**HIGH** — pure additive bug fix in `reindex`. No schema migration
needed. No public API change. Worst case: trim is too aggressive and
deletes a row whose file was temporarily unreadable; user can re-run
ingest or manually re-add. The fix should add an explicit confirmation
flag (`--trim-orphans` opt-in OR `--no-trim` opt-out) to control the
behavior, with sane default (probably opt-in for one release, then
default-on after observation).

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-003 | informs (files-as-source-of-truth principle) |
| PROB-027 | sibling (related reindex bug — cannot rebuild from scratch when lance dir missing) |
| PRD-043 | sibling (methodology integrity, this is data integrity) |
| EPIC-003 | context (found during v0.17.0 final dogfood audit 2026-04-08) |

