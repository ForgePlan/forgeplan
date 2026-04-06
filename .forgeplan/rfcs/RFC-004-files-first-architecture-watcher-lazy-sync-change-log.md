---
depth: standard
id: RFC-004
kind: rfc
links:
- target: ADR-003
  relation: based_on
- target: PROB-017
  relation: informs
- target: RFC-003
  relation: refines
status: active
title: Files-First Architecture — watcher, lazy sync, change log
---

# RFC-004: Files-First Architecture — watcher, lazy sync, change log

## Summary

Invert source of truth: .md files become authoritative, LanceDB becomes index/cache.
Lazy sync before commands + file watcher daemon + change log + git integration.
Eliminates data loss from file edits and enables git-native workflow.

## Problem

LanceDB = source of truth, .md files = read-only projections. This causes:
1. **Data loss**: editing .md file, then running any forgeplan command → file overwritten with LanceDB template
2. **Sync impossibility**: users/agents cannot edit files directly
3. **Git-unfriendly**: git diff/merge/review don't show full picture
4. **Fragile reinit**: rm -rf .forgeplan/lance = total data loss

Observed in production: PROB-017 body lost after `forgeplan link` (2026-03-31).

## Decision

**Invert source of truth**: .md files = primary, LanceDB = index/cache.

## Architecture

```
.forgeplan/prds/PRD-001.md  ← AUTHORITATIVE
       │
       ├── [forgeplan new/update/link] → writes to .md file
       ├── [user/agent edits] → writes to .md file directly
       │
       ▼ (sync layer)
       │
       ├── Phase 1: lazy_sync() before each command
       ├── Phase 2: forgeplan watch (notify daemon)
       ├── Phase 4: git-aware sync
       │
       ▼
LanceDB (index: search, vectors, graph queries)
change_log table (audit trail)
```

### Data Flow

| Action | What happens |
|--------|-------------|
| `forgeplan new prd "T"` | Creates .md file → sync → LanceDB indexed |
| `forgeplan update X` | Modifies .md file → sync → LanceDB updated |
| `forgeplan link A B` | Adds to frontmatter `related:` in both .md → sync → LanceDB |
| User edits .md | File changed → lazy_sync/watcher detects → LanceDB updated |
| `git pull` | Files changed → watcher/lazy_sync → LanceDB updated |
| `rm -rf lance/` | `forgeplan reindex` rebuilds from .md files (zero data loss) |

### What goes where

| Data | Stored in | Format |
|------|-----------|--------|
| Artifact content | .md file (frontmatter + body) | YAML + Markdown |
| Relations/links | frontmatter `related:` field | YAML array |
| Status, depth, kind | frontmatter fields | YAML |
| Embeddings | LanceDB (cache) | f32 vectors |
| R_eff score | Computed on-the-fly | Not stored |
| Change log | LanceDB change_log table | Structured rows |

### Frontmatter schema (after migration)

```yaml
---
id: PRD-001
title: "Auth System"
kind: prd
status: active
depth: standard
author: user
created: 2026-03-31
updated: 2026-03-31
related:
  - target: RFC-001
    relation: based_on
  - target: EVID-001
    relation: informs
---
```

### Change Log Table

| Column | Type | Description |
|--------|------|-------------|
| timestamp | String (RFC3339) | When change happened |
| artifact_id | String | Which artifact |
| action | String | create/update/delete/link/unlink |
| field | String (nullable) | Which field changed (status, body, title...) |
| old_value | String (nullable) | Previous value (hash for body) |
| new_value | String (nullable) | New value (hash for body) |
| source | String | cli / file_edit / git_sync / reindex |

### New Commands

| Command | Description |
|---------|-------------|
| `forgeplan watch` | Start file watcher daemon |
| `forgeplan log [id]` | Show change history |
| `forgeplan log --since "2d"` | Filter by time |
| `forgeplan log --source file_edit` | Filter by source |
| `forgeplan reindex` | Full rebuild LanceDB from .md files |

## Implementation Phases

- [ ] **Phase 1: Lazy Sync** (~100 LOC, 1 sprint)
  - Before each command: compare file mtime vs LanceDB updated_at
  - If file newer → parse .md → update LanceDB body + frontmatter fields
  - If LanceDB newer (after CLI write) → no action needed
  - Prevents data loss on file edit + subsequent command
  - New: `forgeplan reindex` — full rebuild from files

- [ ] **Phase 2: File Watcher** (~200 LOC, 1 sprint)
  - `forgeplan watch` command — background daemon
  - Uses `notify` crate (v9.0) for cross-platform file watching
  - Debounce 500ms (batch rapid saves)
  - On change: parse → diff → update LanceDB → write change_log

- [ ] **Phase 3: Change Log** (~150 LOC, 1 sprint)
  - New LanceDB table: change_log
  - `forgeplan log` CLI command
  - Track: creates, updates, deletes, link changes
  - Source tracking: cli vs file_edit vs git_sync

- [ ] **Phase 4: Git Integration** (~100 LOC, 1 sprint)
  - Detect changes from `git pull/merge`
  - Record commit hash in change_log
  - `forgeplan log --source git_sync`
  - Conflict detection: file changed in both git and locally

- [ ] **Phase 5: Migration** (~80 LOC, 1 sprint)
  - Migrate existing: write current LanceDB body → .md files (one-time)
  - Add `related:` to frontmatter from relations table
  - Verify round-trip: export → reindex → compare

## Risks

| Risk | Mitigation |
|------|-----------|
| Watcher misses changes | `forgeplan reindex` as manual fallback |
| Concurrent writes (two editors) | Last-write-wins (same as git) |
| Large workspace (1000+ artifacts) | Lazy sync = O(1) per command, reindex = O(N) |
| Frontmatter parse errors | Validate on sync, log error, skip corrupt file |

## Dependencies

- `notify` 9.0 crate (file watching, cross-platform)
- `ignore` 0.4 crate (gitignore-aware walking for reindex)
- Existing: frontmatter parser, LanceDB store

## Non-Goals

- Real-time collaboration (not needed for local-first tool)
- Distributed sync (git handles this)
- Automatic conflict resolution (user resolves via git)

## Related Artifacts

| Artifact | Relation |
|----------|----------|
| ADR-003 | based_on |
| PROB-017 | informs |
| RFC-003 | refines |


