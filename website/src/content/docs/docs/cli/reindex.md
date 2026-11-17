---
title: forgeplan reindex
description: "Rebuild LanceDB index from .md files (files-first sync, ADR-003)"
---

Synchronize the LanceDB index from artifact Markdown in `.forgeplan/` without writing those Markdown sources. This is the primary recovery command for a checkout with a missing index, not a promise to replay every piece of local workspace state.

## Usage

```text
forgeplan reindex
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## What it does

1. Reads Markdown in artifact directories and initializes a missing index.
2. Restores supported frontmatter fields and body for new records. For an existing ID, synchronizes the body only: manually changing status, title, or other metadata does not guarantee that the stored record changes.
3. Adds missing frontmatter relations, but does not remove an existing edge merely because its link disappeared from the file. Use `unlink` for removal and the appropriate CLI command for metadata.
4. Prunes records with no corresponding file or a corrupt kind, and relations with missing endpoints.
5. Does not rescore R_eff or restore vectors, event history, or a full snapshot of every table. Run `score --all` and `embed` separately when needed.

## When to run

- **After `git clone`** of an existing workspace: run `forgeplan reindex`; a separate init is not needed to create the missing index.
- **After manual body edits**: reindex synchronizes changed bodies. Use the CLI for metadata and relation removals as explained above.
- **After `git pull`** that brought in new artifacts from teammates - see also
  [`forgeplan git-sync`](/docs/cli/git-sync/) for the incremental variant.
- **Recovery or upgrade**: retain the Markdown sources and run reindex. A damaged database that cannot be opened can still make reindex fail; the command does not replace arbitrary corrupt state or replay history.

## Example

```bash
# In an existing checkout containing tracked .forgeplan Markdown:
forgeplan reindex
forgeplan list
# After editing an actual artifact body in your editor:
forgeplan reindex
forgeplan search "auth"
```

## Safety

`reindex` leaves Markdown untouched but mutates and prunes the index. Use a complete checkout and inspect the summary, warnings, per-file errors, and ID collisions; the command can report per-file errors without a failing process exit. Do not treat exit 0 alone as a clean reconstruction or delete the index as a routine prerequisite.

## v0.18.0 note (PROB-027)

Prior to v0.18.0, `reindex` assumed `.forgeplan/lance/` already existed and
could fail on fresh clones. As of v0.18.0 the command bootstraps the index
directory if missing, so the "clone → reindex" workflow now works end-to-end
without a separate `init` step on existing workspaces.

## Orphan relation cleanup (v0.17.1 hotfix)

As of v0.17.1 (PROB-028, PRD-044) `reindex` runs a **Phase 3** pass that trims
orphan relations - links in the `relations` table whose source or target
artifact no longer exists in the `artifacts` table.

Before v0.17.1, deleting or deprecating an artifact left its relation rows
behind in LanceDB. These orphan links caused phantom `?` rows in
`forgeplan tree` output and inflated link counts in `forgeplan health`. Phase 3
removes them and prints a counter:

```
Reindex complete: 147 artifacts, 3 removed (corrupt kind), 5 orphan relations trimmed
```

Removal reasons reported in the output:

| Reason                         | What happened                                    |
|--------------------------------|--------------------------------------------------|
| `corrupt kind field`           | Row with unparseable kind - no valid directory    |
| `no .md file found`            | LanceDB row with no corresponding markdown file   |
| `orphan relation (source missing)` | Relation pointing from a deleted artifact     |
| `orphan relation (target missing)` | Relation pointing to a deleted artifact      |
| `orphan relation (both missing)`   | Both ends of the link are gone               |

### When to run

- **After bulk deletes** - if you removed several `.md` files manually.
- **After manual `.md` removes** - `rm .forgeplan/prds/prd-old.md` leaves a
  dangling LanceDB row until you reindex.
- **Session start after `git pull`** - teammates may have deprecated or deleted
  artifacts on their branches.
- **After upgrade from pre-v0.17.1** - one reindex cleans up any accumulated
  orphan relations from the pre-fix era.

## `reindex` vs `scan-import`

- **`reindex`** - targeted rebuild of LanceDB from current markdown state.
  Use in normal day-to-day work.
- **[`scan-import`](/docs/cli/scan-import/)** - onboarding importer for external Markdown, not a refresh of existing artifact bodies. Prefer
  `reindex` unless you're doing a one-time import or migration.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan scan-import`](/docs/cli/scan-import/) - import external Markdown
- [`forgeplan git-sync`](/docs/cli/git-sync/) - incremental sync after pull
- [`forgeplan watch`](/docs/cli/watch/) - continuous auto-sync daemon
- [`forgeplan health`](/docs/cli/health/) - verify index integrity
