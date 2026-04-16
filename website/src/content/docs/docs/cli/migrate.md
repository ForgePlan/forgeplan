---
title: forgeplan migrate
description: "Apply schema migrations to an existing workspace after upgrading the Forgeplan binary"
---

`forgeplan migrate` runs pending schema migrations against the LanceDB tables in `.forgeplan/lance/`. Forgeplan occasionally adds columns or changes table structure between releases (e.g. v0.17 → v0.18 added columns for BM25 and Russian morphology). `migrate` applies those changes in-place, preserving all artifacts, links, and evidence, without forcing a destructive reinit.

## When to use

- Immediately after upgrading the Forgeplan binary (`cargo install forgeplan`, `brew upgrade forgeplan`, or a release download).
- When `forgeplan health` or any other command reports schema version mismatch or "missing column" errors.
- Before resuming work in a workspace that was last touched under an older version.

## When NOT to use

- On a brand-new workspace created by the current binary — there is nothing to migrate.
- As a substitute for rebuilding the search index — migrations change schema, not content. For index rebuilds use [`forgeplan scan-import`](/docs/cli/scan-import/).
- As a recovery tool for corrupted workspaces — migrations assume the LanceDB files are structurally valid. For corruption, restore from [`forgeplan export`](/docs/cli/export/) backup.

## Usage

```text
forgeplan migrate
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: Post-upgrade migration

```bash
cargo install forgeplan        # upgrade to new version
forgeplan migrate              # apply any pending schema changes
forgeplan health               # verify workspace is clean
```

The standard post-upgrade sequence. `migrate` is a no-op if nothing changed, so it's safe to run unconditionally after every upgrade.

### Example 2: Fixing "missing column" errors

```bash
forgeplan list
# Error: column 'bm25_tokens' not found
forgeplan migrate
forgeplan list
# OK
```

If a command fails with a schema-level error, `migrate` is usually the one-step fix.

### Example 3: Safe rollback path (if migrate is not enough)

```bash
forgeplan export --output pre-migrate-backup.json
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
forgeplan migrate
# if something goes wrong:
rm -rf .forgeplan
forgeplan init -y
forgeplan import pre-migrate-backup.json
```

Always back up before running migrations against production workspaces with meaningful artifact history.

## How it fits the workflow

`migrate` is part of the upgrade path, not the daily artifact loop. A typical version bump goes:

1. `forgeplan export --output pre-upgrade.json` — safety backup
2. Upgrade the binary (`cargo install` / `brew upgrade`)
3. `forgeplan migrate` — apply pending schema changes
4. `forgeplan health` — verify the workspace is clean
5. Resume normal work (`forgeplan new`, `forgeplan validate`, etc.)

If `migrate` is insufficient (e.g. a breaking schema change), fall back to the export + reinit + import cycle documented in [`forgeplan init`](/docs/cli/init/).

## Safety notes

- **Always export before migrating production workspaces.** `migrate` is designed to be non-destructive, but new binaries can have bugs. [`forgeplan export`](/docs/cli/export/) is cheap and fast; run it first.
- **`migrate` is idempotent.** Running it twice in a row is safe — the second run detects no pending migrations.
- **`.forgeplan/lance/` is gitignored** — if migration leaves it in a weird state on one machine, `rm -rf .forgeplan/lance && forgeplan scan-import` will rebuild the index from tracked markdown.
- **Schema version is tied to the binary.** Don't mix binaries of different versions against the same workspace without running `migrate` in between.

## See also

- [`forgeplan export`](/docs/cli/export/) — mandatory safety backup before upgrading
- [`forgeplan import`](/docs/cli/import/) — rollback path if migration fails
- [`forgeplan init`](/docs/cli/init/) — last-resort full reinit
- [`forgeplan scan-import`](/docs/cli/scan-import/) — rebuild the derived index from markdown
- [`forgeplan health`](/docs/cli/health/) — post-migration verification
