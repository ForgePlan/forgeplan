---
depth: standard
id: PRD-044
kind: prd
links:
- target: PROB-028
  relation: based_on
- target: ADR-003
  relation: informs
status: draft
title: Reindex trim orphans — delete LanceDB rows without .md backing file (v0.17.1 hotfix)
---

# PRD-044: Reindex trim orphans (v0.17.1 hotfix)

## Progress

```
FR-001   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  Fix parse-kind bug: treat corrupt kind as orphan, not skip
FR-002   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  Improve trim output message (reason: corrupt-kind vs missing-file)
FR-003   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  Add --show-orphans flag: git log + recovery recipe per orphan
FR-004   ░░░░░░░░░░░░░░░░░░░░░░░░  0/1  Update --help text, CLAUDE.md, CHANGELOG.md
─────────────────────────────────────────────────
TOTAL                               0/4  (  0%)
```

## ADI revision note (2026-04-09)

Initial PRD spec was Option D (warn by default, add --trim-orphans flag).
ADI code investigation revealed the root cause is **different** from the
initial hypothesis: reindex **already trims by default** in Phase 2
(reindex.rs:134-175). The real bug is at lines 138-141:

```rust
for record in &all_records {
    let kind: ArtifactKind = match record.kind.parse() {
        Ok(k) => k,
        Err(_) => continue,  // ← SKIPS CORRUPT ROWS FROM TRIM
    };
    // ... check if file exists ...
}
```

NOTE-037 and NOTE-040 have corrupt/empty `kind` field (observed `?` in
`forgeplan tree` output). `extract_record` at store.rs:1390 uses
`unwrap_or_default()` so null kind becomes `""`. `"".parse::<ArtifactKind>()`
returns Err. `continue` skips the row. Result: rows with corrupt kind
**escape trim forever**.

**Revised scope** (H2 from ADI): minimal bug fix, not a new feature.
Change `Err(_) => continue` to treat unparseable kind as definite orphan
(no valid kind = no valid directory = no possible file = trim it).
Keep existing default-trim behavior. Add --show-orphans as additive
feature only (no change to default).

**Rejected original Option D**: would have CHANGED default from trim to
warn, which is a regression for users relying on auto-cleanup. The bug
is in trim, not absence of trim.

## Problem

`forgeplan reindex` is currently one-way: it walks `.forgeplan/*/*.md`
files and upserts them into LanceDB. It does NOT trim LanceDB rows
whose `.md` file was deleted. Result: **phantom rows** accumulate
whenever a user deletes an `.md` file externally (git pull, manual
cleanup, `mv`).

Observed in dogfood workspace (2026-04-08): NOTE-037 and NOTE-040
show as `?` phantoms in `forgeplan tree` output. `forgeplan get
NOTE-037` returns "not found" but the row exists in
`.forgeplan/lance/artifacts.lance/` binary data.

This is a passive violation of **ADR-003** (files = source of truth,
LanceDB = derived index). The index should be fully reproducible
from files; when the index has rows that files don't, the invariant
is broken.

## Goals

| ID | Goal | Metric |
|----|------|--------|
| G-1 | reindex trims LanceDB rows that have no backing `.md` file | After `forgeplan reindex` on a workspace with deleted files, `tree` shows 0 phantom rows |
| G-2 | Trim is observable — users see how many rows were removed | reindex output line: "Reindex complete: N synced, M unchanged, K trimmed, L errors" |
| G-3 | Safe — doesn't delete rows when file is merely unreadable due to transient I/O | Errors during file stat → don't trim, log warning |
| G-4 | Idempotent | Second reindex after successful trim shows "0 trimmed" |

## Non-Goals

- Real-time file watching (already exists as `forgeplan watch`, separate concern)
- Soft-delete / recoverable trash for trimmed rows (if users wanted history, they should use git)
- Trimming rows that reference artifacts with `status: deprecated` (deprecation preserves record)
- Rebuilding relations table (separate concern — if source of a relation is trimmed, the relation should cascade, but that's FR-scope for a future PRD)

## Target Users

| Persona | Pain |
|---|---|
| Developer doing `git pull` that deletes artifacts from another branch | Phantom rows accumulate silently, pollute `tree`/`list` |
| Team doing dogfood cleanup (delete old experimental artifacts) | `reindex` doesn't clean up, manual LanceDB surgery required |
| AI agent using MCP `forgeplan_list` tool | Sees stale IDs that don't resolve — breaks workflow |

## User Journeys

### Journey 1: Developer deletes artifact via git
1. Developer on branch A: `rm .forgeplan/notes/NOTE-037.md && git commit && git push`
2. Developer on branch B: `git pull`  — their workspace now has LanceDB row for NOTE-037 but no file
3. `forgeplan tree` — phantom NOTE-037 appears
4. **NEW**: `forgeplan reindex` — prints "Reindex complete: 0 synced, 165 unchanged, 1 trimmed, 0 errors"
5. `forgeplan tree` — clean, no phantom

### Journey 2: Dogfood cleanup
1. Maintainer: `rm .forgeplan/evidence/EVID-{003,004,007,008,010}.md`
2. `forgeplan reindex` → 5 trimmed
3. `forgeplan health` shows "Possible duplicates: 0" (the duplicates pairs resolved because one side of each pair was trimmed)

## Functional Requirements

- **FR-001** Core Must. [System] Phase 2 cleanup in `reindex` MUST trim
  rows whose `kind` field fails to parse, treating unparseable kind as
  a definite orphan (no valid kind means no valid directory, so no file
  could exist). Current code skips these via `continue`; after fix they
  must be deleted along with regular orphans.
- **FR-002** UX Must. [User] sees improved reindex output distinguishing
  removal reasons. Two cases: `DEL ID — file deleted` (normal case) and
  `DEL ID — corrupt kind field` (phantom case). Total count line remains
  `Reindex complete: N synced, M unchanged, K removed, L errors`.
- **FR-003** Feature Must. [User] can pass `--show-orphans` flag to
  `forgeplan reindex` to preview what would be deleted BEFORE the trim
  runs. For each orphan: print ID, last known title from LanceDB, and
  `git log --all -- [path]` output showing last commit SHA, date, author,
  and message. Also print copy-paste recovery recipe using those values.
  With this flag, trim still happens (unless combined with `--dry-run`
  if that exists — out of scope if not).
- **FR-004** Docs Must. [Contributor] finds up-to-date documentation for
  the new flag and fixed behavior: clap `#[arg(help)]` string on
  `--show-orphans`, CHANGELOG.md v0.17.1 Fixed entry for the bug and
  Added entry for the flag, CLAUDE.md reindex workflow mention.
  **No feature lands without help text + changelog** (NOTE-044 rule).

## Non-Functional Requirements

| ID | Category | Requirement |
|----|----------|-------------|
| NFR-001 | Performance | Trim pass adds O(N) complexity where N = rows in LanceDB; must not exceed 2× baseline reindex time on 1k-artifact workspace |
| NFR-002 | Safety | Trim MUST check file existence per-row; a transient I/O error on file stat MUST skip that row (do not delete), not fail the whole reindex |
| NFR-003 | Backward compat | Default behavior of `forgeplan reindex` must not cause data loss for users who did NOT intend to trim |

## Design decision — RESOLVED: Option D

**Decision date**: 2026-04-09
**Decided by**: gogocat (project owner)

### Chosen: Option D — Warn-only default + two helper flags

Reindex by default **warns** about phantom rows but does not delete.
Two opt-in flags handle removal and inspection:

- `forgeplan reindex` (default) — detects phantoms, prints warning
  block with list and recovery hints. No mutation.
- `forgeplan reindex --trim-orphans` — performs hard-delete of
  phantom rows from LanceDB.
- `forgeplan reindex --show-orphans` — for each phantom, runs
  `git log --all -- [path]` to show last commit, date, author,
  message. Lets user review "what was in here" before trimming.

### Why Option D (not A/B/C)

User requested "warn, but also support soft-recovery" initially.
Reason-mode analysis compared soft-delete (trash bucket in LanceDB)
vs git history as the recovery mechanism and picked git for these
reasons:

1. **ADR-003 compliance** — files are source of truth, LanceDB is
   derived. Adding a "trash" table makes LanceDB carry state that
   isn't in files, breaking the invariant.
2. **Zero new architecture** — git already stores history, retention,
   restore, cross-developer sync. Soft-delete would duplicate all of
   this inside LanceDB.
3. **Scope fits hotfix** — Option D is 5 FRs, ~100 LOC total.
   Soft-delete would be 5+ FRs + schema migration + retention policy
   + restore command = a week+ of work, not a hotfix.
4. **Better recovery story** — `git log -p` shows what *changed* in
   the artifact over time, not just the last version. Richer history
   than any internal trash.

### How recovery works in Option D

Scenario: User accidentally deleted `NOTE-037.md`, ran `reindex
--trim-orphans`, now regrets it.

```bash
# 1. Find when NOTE-037 was deleted
git log --all --diff-filter=D --summary -- .forgeplan/notes/NOTE-037*
# → commit abc123f dropped NOTE-037-title.md

# 2. Checkout the file from the commit just before deletion
git show abc123f~1:.forgeplan/notes/NOTE-037-title.md \
  > .forgeplan/notes/NOTE-037-title.md

# 3. Reindex to add it back to LanceDB
forgeplan reindex
```

The `--show-orphans` flag will print exactly these commands as part
of its output, so users don't need to know git spelunking.

### Operational safety

- Default is WARN ONLY — no user will lose data by running `reindex`
- `--trim-orphans` is a loud, explicit action (no short form, no
  config default)
- Warning includes both hint lines, so the `--show-orphans` path is
  discoverable even without reading docs
- `.forgeplan/lance/` is gitignored, so trim only affects local index
  — pulling from git will never restore trimmed rows accidentally;
  user must run reindex explicitly after pulling

### Rejected alternatives

- **Option A** (default-on trim) — would silently lose data in
  `git stash` edge case, violates patch-release convention
- **Option B** (default-off opt-in flag in v0.17.1, flip to on in
  v0.18) — needed to create a follow-up PRD for the flip, user never
  gets the fix unless they read release notes
- **Option C** (warn-only, no helper) — pure warn is fine but lacks
  the recovery UX. `--show-orphans` via git closes this gap cheaply.
- **Soft delete (trash bucket)** — 5+ additional FRs, schema migration,
  retention policy, restore command. Not a hotfix. Git already does this.

## Acceptance Criteria

- **FR-001**: `reindex` detects LanceDB rows whose ID has no `.md`
  file in the expected kind directory. Read-only scan, no mutation.
- **FR-002**: Default `reindex` prints a warning block listing phantom
  IDs and two hint lines (`--trim-orphans` to remove, `--show-orphans`
  to inspect via git). Exit code 0 — warning is informational.
- **FR-003**: `reindex --trim-orphans` performs hard-delete of all
  detected phantom rows. Output line includes trimmed count:
  "Reindex complete: N synced, M unchanged, K trimmed, L errors".
- **FR-004**: `reindex --show-orphans` runs `git log --all -- [file]`
  per phantom, printing last commit SHA, date, author, and message.
  Also prints the concrete recovery recipe from the "How recovery
  works" section above, parameterized with the actual paths.
- **FR-005**: Help text updated — clap `#[arg(help)]` strings for
  both new flags, CHANGELOG.md v0.17.1 entry under "Added" section,
  CLAUDE.md reindex workflow mention, `docs/methodology/FORGEPLAN-GUIDE.md`
  reindex subsection updated.
- **All existing reindex tests pass** unchanged.
- **New test: detect phantom** — create artifact, delete its `.md`,
  reindex without flags, assert warning block printed, row NOT deleted.
- **New test: trim phantom** — same setup but reindex with
  `--trim-orphans`, assert row removed and `forgeplan get` returns
  not-found, second reindex trims 0 (idempotency).
- **New test: show orphans** — same setup with `--show-orphans`,
  assert output contains git commit SHA + recovery recipe.
- **New test: I/O error resilience** — simulate stat failure on one
  file, assert trim of OTHER legitimately-orphan rows proceeds
  (graceful degradation).

## Affected Files

- `crates/forgeplan-core/src/db/store.rs` — add `trim_orphans()` method
- `crates/forgeplan-cli/src/commands/reindex.rs` — wire flag + call trim
- `crates/forgeplan-cli/src/main.rs` — add flag to ReindexCommand clap
- `crates/forgeplan-core/src/db/store.rs` — tests module
- `crates/forgeplan-cli/tests/reindex_test.rs` (NEW or extend existing)

## Related

| Artifact | Relation |
|---|---|
| PROB-028 | based_on (this PRD closes PROB-028) |
| ADR-003 | informs (files = source of truth, PRD restores this invariant) |
| PROB-027 | sibling (other reindex bug — cannot rebuild from scratch) |
| PRD-045 | sibling (health verdict fix, paired v0.17.1 hotfix work) |
