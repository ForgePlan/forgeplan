---
title: forgeplan migrate
description: "Apply schema migrations to an existing workspace after upgrading the Forgeplan binary"
---

`forgeplan migrate` opens the legacy LanceDB store, which runs pending schema migrations. It is not Constellation layout migration. The selected build refuses a Constellation-owned layout before opening the legacy store; preserve its markers and use its documented `index` flow.

## When to use

- Immediately after upgrading the Forgeplan binary (`cargo install forgeplan`, `brew upgrade forgeplan`, or a release download).
- When `forgeplan health` or any other command reports schema version mismatch or "missing column" errors.
- Before resuming work in a workspace that was last touched under an older version.

## When NOT to use

- On a brand-new workspace created by the current binary - there is nothing to migrate.
- As a content rebuild - use the appropriate `reindex`/`index` path. `scan-import` imports external Markdown, including on migrated workspaces; it is not a canonical index rebuild.
- As a recovery tool for corrupted workspaces - migrations assume the LanceDB files are structurally valid. For corruption, restore from [`forgeplan export`](/docs/cli/export/) backup.

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

This is a legacy-layout upgrade example. Check the binary and layout first; help registration is not a compatibility guarantee.

### Example 2: Fixing "missing column" errors

```bash
forgeplan list
# Error: column 'bm25_tokens' not found
forgeplan migrate
forgeplan list
# OK
```

A schema error needs diagnosis against the installed version; this command is not a guaranteed one-step fix.

### Example 3: Safe rollback path (if migrate is not enough)

```bash
forgeplan export --output pre-migrate-backup.json
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
forgeplan migrate
# If migration fails, stop and retain the original plus its backup.
# Diagnose the version/layout mismatch before selecting a recovery procedure.
```

Always back up before running migrations against production workspaces with meaningful artifact history.

## How it fits the workflow

`migrate` is part of the upgrade path, not the daily artifact loop. A typical version bump goes:

1. `forgeplan export --output pre-upgrade.json` - safety backup
2. Upgrade the binary (`cargo install` / `brew upgrade`)
3. `forgeplan migrate` - apply pending schema changes
4. `forgeplan health` - verify the workspace is clean
5. Resume normal work (`forgeplan new`, `forgeplan validate`, etc.)

If migration fails, preserve the workspace and backup. Reinitialization is not a generic rollback or Constellation recovery path.

## Safety notes

- **Always export before migrating production workspaces.** `migrate` is designed to be non-destructive, but new binaries can have bugs. [`forgeplan export`](/docs/cli/export/) is cheap and fast; run it first.
- **`migrate` is idempotent.** Running it twice in a row is safe - the second run detects no pending migrations.
- **Preserve `.forgeplan/lance/` for diagnosis.** Do not delete protocol markers or retired-table backups. `scan-import` is not an index rebuild.
- **Use the binary intended for this layout.** Legacy migration does not make arbitrary versions compatible.

## See also

- [`forgeplan export`](/docs/cli/export/) - mandatory safety backup before upgrading
- [`forgeplan import`](/docs/cli/import/) - rollback path if migration fails
- [`forgeplan init`](/docs/cli/init/) - deliberate initialization/config replacement
- [`forgeplan reindex`](/docs/cli/reindex/) - layout-specific reconciliation
- [`forgeplan health`](/docs/cli/health/) - post-migration verification

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan migrate --help`.

```text
Run schema migrations on existing workspace

Usage: forgeplan migrate

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
