---
title: forgeplan reindex
description: "Rebuild LanceDB index from .md files (files-first sync, ADR-003)"
---

Rebuild the LanceDB search and metadata index from the markdown files in
`.forgeplan/`. This is a read-only operation against your markdown sources -
the `.md` files are the source of truth (ADR-003), and `lance/` is a derived,
gitignored cache that can always be recomputed.

In local `0.39.0+constellation.s1.1`, a Constellation-owned layout routes
`reindex` through the same reconcile path as `forgeplan index`; it does not
recreate legacy artifact tables. Configured embedding maintenance can also
drain pending vectors. The legacy table description below applies only to
the legacy layout. See [Constellation](/docs/guides/constellation/).

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

1. Scans every artifact directory under `.forgeplan/` (adrs, rfcs, prds, epics,
   specs, evidence, problems, solutions, notes, refresh).
2. Parses YAML frontmatter + markdown body of each file.
3. Upserts artifacts and frontmatter relations, then trims missing-file artifact
   rows and orphan rows in the legacy `relations` table.
4. Does not promise to recreate every auxiliary table or recompute every score.
5. Leaves your markdown files untouched.

## When to run

- **After `git clone`** - verify the intended root first. `init -y` does
  nothing if `.forgeplan/` exists, including in a parent. If config is
  missing and defaults are intended, use `init -y --force` with its backup,
  restore custom settings, then `reindex`.
- **After manual markdown edits** - if you edited PRD/RFC files outside the
  CLI (text editor, find-and-replace), `reindex` syncs the changes into
  LanceDB so `search`, `list`, and `health` see them.
- **After `git pull`** that brought in new artifacts from teammates - see also
  [`forgeplan git-sync`](/docs/cli/git-sync/) for the incremental variant.
- **Recovery from corruption** - if `lance/` is damaged or LanceDB schema
  migrated, a full reindex rebuilds from markdown truth.
- **Schema migration** - new LanceDB columns in a Forgeplan upgrade require
  reindex to populate them.

## Example

```bash
# Fresh clone bootstrap
git clone <repo> && cd myproject
forgeplan init -y
forgeplan reindex
forgeplan list       # verify

# After editing a PRD in your text editor
vim .forgeplan/prds/prd-001-auth.md
forgeplan reindex
forgeplan search "auth"
```

## Safety

`reindex` preserves canonical `.md` files while changing derived state.
Keep the existing index and model cache for diagnosis rather than deleting
them. Legacy and Constellation layouts are not interchangeable.

## v0.18.0 note (PROB-027)

Historically, v0.18.0 fixed rebuilding a missing legacy `lance/` directory.
That does not remove the current requirement to load workspace configuration,
nor does it make repeated `init -y` repair a missing config.

## Orphan relation cleanup (v0.17.1 hotfix)

As of v0.17.1 (PROB-028, PRD-044) `reindex` runs a **Phase 3** pass that trims
orphan relations - links in the `relations` table whose source or target
artifact no longer exists in the `artifacts` table.

Before v0.17.1, deleting or deprecating an artifact left its relation rows
behind in LanceDB. These orphan links caused phantom `?` rows in
`forgeplan tree` output and inflated link counts in `forgeplan health`. Phase 3
removes them and prints a counter:

Historical illustrative output, not a transcript of the local Constellation build:

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
- **[`scan-import`](/docs/cli/scan-import/)** imports external Markdown, including through the migrated file-first path.
  Use `index`/`reindex` to rebuild the index from existing canonical sources.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan scan-import`](/docs/cli/scan-import/) - bulk import + rescan
- [`forgeplan git-sync`](/docs/cli/git-sync/) - incremental sync after pull
- [`forgeplan watch`](/docs/cli/watch/) - continuous auto-sync daemon
- [`forgeplan health`](/docs/cli/health/) - verify index integrity

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan reindex --help`.

```text
Rebuild LanceDB index from .md files (files-first sync)

Usage: forgeplan reindex

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
