---
title: forgeplan import
description: "Read an export JSON bundle and restore markdown projections, index records, and typed relations into the current workspace. Existing artifact IDs are skipped by default, not treated as fatal conflicts."
---

Read an `export` JSON bundle and restore markdown projections, index records, and typed relations into the current workspace. Existing artifact IDs are skipped by default, not treated as fatal conflicts.

## Usage

```text
forgeplan import [OPTIONS] <PATH>
```

## Arguments

```text
  <PATH>  Path to JSON export file
```

## Options

```text
      --force    Overwrite existing artifacts
  -h, --help     Print help
  -V, --version  Print version
```

## Forgeplan 0.38.0

Import is not one atomic transaction. Unknown kinds fall back to note; statuses outside draft/active/superseded/deprecated fall back to draft with a warning, including exported stale. Active stub content is downgraded to draft unless forced. Individual record failures or unapplied relations can remain in the summary without failing the entire command: inspect imported/skipped/failed and relation counts, then validate/health.

## Overwrite policy and limits

`--force` overwrites existing artifacts and can bypass the imported-active stub guard; review the bundle and keep a backup. Kind changes for an existing ID are refused. Input is limited to 100 MiB. Import writes markdown as well as LanceDB; do not run `scan-import` afterward as reconciliation.

## Examples

```bash
# In a new destination directory:
forgeplan init -y
forgeplan import ../forgeplan-backup.json
forgeplan list
forgeplan health
# Only when the bundle is the intended replacement:
forgeplan import ../forgeplan-backup.json --force
```

## Recovery boundaries

Import restores content, supported metadata, tags, and relations, but does not replay the export’s `r_eff_score`, `created_at`, or `updated_at` fields. Recompute scores with `score --all` and inspect the result with `health`. It does not restore full scoring history, provider settings, secrets, caches, claims, or every local file. Preserve those separately. With intact workspace markdown use `reindex`; for foreign documents use `scan-import`.

## See also

- [`forgeplan export`](/docs/cli/export/) - the other half of the backup pair
- [`forgeplan init`](/docs/cli/init/) - create the destination workspace
- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild from markdown (preferred for fresh clones)
- [`forgeplan migrate`](/docs/cli/migrate/) - non-destructive alternative when only the schema has drifted
- [`forgeplan health`](/docs/cli/health/) - post-import verification
