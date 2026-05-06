---
depth: standard
id: EVID-110
kind: evidence
links:
- target: PROB-028
  relation: informs
status: active
title: PROB-028 closure reindex resilience file_title fix per-file error continue
---

# EVID-110: PROB-028 closure — reindex resilience против Phase-1 abort

## Summary

Closes the practical reachability gap в PROB-028. v0.17.1 introduced Phase 2/3 orphan trim в `forgeplan reindex` (rows whose `.md` file disappeared, и orphan relations cascading from trimmed artifacts). Эта logic существовала, но **не достигалась** — Phase 1 propagated the first per-file error via `?` и aborted the entire reindex. Real-world scenario observed в project workspace today: `forgeplan reindex` errored on a single SESSION-2026-04-06 record (`FileNotFound` from `sync_body_from_file` due к title-divergent-from-DB), aborted, и left 2 stale orphan rows (PRD-001 / SPEC-001) untrimmed.

## Structured Fields

verdict: supports
congruence_level: 3
evidence_type: test

## Evidence

### Two-part fix в `crates/forgeplan-cli/src/commands/reindex.rs`

**Part A** — Pass file's parsed title к `sync_body_from_file` instead of DB-stored `record.title`:

```rust
let file_title = fm.get("title").and_then(|v| v.as_str()).unwrap_or(&record.title);
```

Pre-fix the helper computed path as `<workspace>/<kind>/<id>-<slug(record.title)>.md`. If user manually edited frontmatter `title:` on disk без syncing back to DB, the computed slug мismatched the actual filename → `FileNotFound`. File is right there в `read_dir`, но helper looked for the DB-shaped name.

**Part B** — Per-file errors now log + `errors += 1` + `continue` вместо `?`-abort:

```rust
match projection::sync_body_from_file(...).await {
    Ok(()) => { synced += 1; }
    Err(e) => { eprintln!("  WARN {} — sync failed: {}", id, e); errors += 1; }
}
```

Same shape applied к `sync_artifact_from_file` (Phase 1 create branch). Phase 2 (orphan trim) и Phase 3 (orphan relations) now ALWAYS run after the per-file loop completes, regardless of how many individual files failed.

### Real E2E на project workspace (target/release/forgeplan, 2026-05-06)

Pre-fix:
```
$ forgeplan reindex
  SYNC PRD-008 — body updated from file
  SYNC PRD-012 — body updated from file
Error: file not found for SESSION-2026-04-06 at notes/SESSION-2026-04-06-marathon-session-2026-04-06-full-knowledge-dump.md
$ echo $?
1
$ forgeplan health
  Orphans (2): PRD-001, SPEC-001  ← stale orphans persist
```

Post-fix:
```
$ ./target/release/forgeplan reindex
  SYNC PRD-008 — body updated from file
  SYNC PRD-012 — body updated from file
  WARN SESSION-2026-04-06 — create failed: file not found for SESSION-2026-04-06 at notes/...
  DEL  PRD-001 — no .md file found, removed from DB
  DEL  SPEC-001 — no .md file found, removed from DB
Reindex complete: 5 synced, 290 unchanged, 2 removed, 0 orphan relations, 1 errors.
$ forgeplan health
  → 1. Project looks healthy. Continue implementation.
  Project looks healthy!
```

Phase 2 ran AFTER the per-file error и trimmed both orphan rows. Workspace is now clean (0 orphans).

### Tests (+2 CLI integration tests)

```
test reindex_trims_orphan_after_md_file_deleted ... ok
test reindex_continues_after_per_file_error_and_still_trims_orphans ... ok
```

First test reproduces PROB-028 AC-5 verbatim: создать artifact, удалить `.md`, run reindex, assert row trimmed + `forgeplan get` → not-found. Second test recreates the bug shape from project workspace: one orphan + one title-divergent file → Phase 1 logs error, Phase 2 still trims orphan.

### Quality gates

```
cargo fmt --check                                                  clean
cargo clippy --workspace --all-targets --features test-helpers
  -- -D warnings                                                   clean
cargo test --workspace --features test-helpers                     0 failures (38 suites)
```

### AC tracking (PROB-028)

- AC-1 ✅ already в v0.17.1 — `reindex` deletes orphan rows
- AC-2 ✅ already — reports trimmed count in summary line
- AC-3 ✅ now reachable — orphans actually disappear after per-file error (was blocked pre-fix)
- AC-4 ✅ idempotent — running again reports 0 trimmed
- AC-5 ✅ +2 integration tests cover deletion-then-reindex AND per-file-error continue
- AC-6 ✅ cross-kind safety preserved (no changes to Phase 2 directory-walk logic)

## Hindsight

PROB-028 has been "active" since 2026-04-08. v0.17.1 shipped what looked like a fix (Phase 2/3 trim logic) but the trim path was unreachable on workspaces with ANY title-divergent record. The bug class is **dependency on Phase 1 success for Phase 2/3 to run** — a pipeline order issue invisible without an actual test exercising both phases в combination.

Lesson: when introducing a new pipeline phase, audit ALL paths through which the previous phase can short-circuit. `?` propagation in a `for` loop is the most common offender — converting к `match … { Ok ⇒ continue; Err ⇒ log + continue }` is the pattern. Mirror of PROB-049 typed-errors lineage (graceful degradation over fail-fast).

Note also: the project workspace exposed this bug **сегодня** through orphan PRD-001 / SPEC-001 from a scan-import smoke test earlier в the session. PROB-028 has been passively observable for weeks — only became an action item when the orphans appeared in `forgeplan health` output during PROB-051 review. **Workspace dogfood matters** — bugs hide in long-running workspaces that fresh-init smoke tests miss.

## Related Artifacts

| Artifact | Relation |
|---|---|
| PROB-028 | informs (this evidence demonstrates closure of the practical reachability gap) |
| PROB-027 | informs (sibling reindex bug — already addressed via init() fix) |
| ADR-003 | informs (files = source of truth principle) |



