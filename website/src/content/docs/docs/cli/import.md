---
title: forgeplan import
description: "Restore artifacts from a JSON export file — reinit recovery, backups, and workspace migration"
---

`forgeplan import` reads a JSON file produced by [`forgeplan export`](/docs/cli/export/) and loads every artifact, link, and evidence record back into the current workspace's LanceDB tables. It is the recovery half of the export/import backup pair and the only supported way to restore state after a destructive reinit.

## When to use

- Restoring a workspace after `rm -rf .forgeplan && forgeplan init -y` (you did export first, right?).
- Moving a workspace between machines when git-tracked markdown alone is not enough (e.g. preserving full scoring history).
- Rolling back a failed `forgeplan migrate` by reinitializing and importing the pre-migration backup.
- Cloning a teammate's workspace state for debugging or reproducing a bug.

## When NOT to use

- To merge two live workspaces — `import` is a restore, not a merge. Conflicts require `--force` and can overwrite good data.
- To rebuild the LanceDB index from markdown — that's [`forgeplan scan-import`](/docs/cli/scan-import/), which is safer because markdown is the source of truth (ADR-003).
- To import data from another tool — Forgeplan's JSON schema is internal. Only files written by `forgeplan export` are supported.

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

## Examples

### Example 1: Restore after reinit

```bash
forgeplan export --output backup.json
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
rm -rf .forgeplan
forgeplan init -y
forgeplan import backup.json
forgeplan health
```

The canonical disaster-recovery cycle. `import` rebuilds every artifact, link, and evidence record from the JSON backup.

### Example 2: Force-overwrite on conflict

```bash
forgeplan import backup.json --force
```

Without `--force`, artifacts that already exist in the workspace cause the import to fail safely. Use `--force` only when you are sure the backup is the source of truth.

### Example 3: Cross-machine workspace transfer

```bash
# on machine A
forgeplan export --output workspace.json
scp workspace.json machine-b:/tmp/

# on machine B
forgeplan init -y
forgeplan import /tmp/workspace.json
forgeplan list
```

Useful when markdown alone is not enough (e.g. you want scoring history, decay state, or links that are only stored in LanceDB).

## How it fits the workflow

`import` is a recovery / migration tool, not part of the daily Shape → Validate → Code → Evidence → Activate cycle. It pairs tightly with `export`:

1. **Before any destructive operation**: `forgeplan export --output backup.json`
2. **After reinit or migration rollback**: `forgeplan import backup.json`
3. **Verify**: `forgeplan health` should show the same artifacts, links, and scores as before

For routine index rebuilds (after `git clone`, for example), prefer [`forgeplan scan-import`](/docs/cli/scan-import/) — it reads markdown, which is the source of truth.

## Safety notes

- **Always `forgeplan health` after import.** Confirm artifact counts, link integrity, and R_eff scores match the pre-export state.
- **`--force` is destructive.** It silently overwrites existing artifacts. If in doubt, import into a fresh `init -y` workspace instead.
- **The JSON format is internal and versioned.** Importing a backup from a much older binary into a newer workspace may require an intermediate `forgeplan migrate` step.
- **Never edit export JSON by hand.** It's not a config file; hand-editing will break checksums and relations.
- **`import` touches LanceDB only.** It does not rewrite the markdown files under `.forgeplan/adrs/`, `prds/`, etc. If markdown and JSON disagree, run `scan-import` afterward to reconcile.

## See also

- [`forgeplan export`](/docs/cli/export/) — the other half of the backup pair
- [`forgeplan init`](/docs/cli/init/) — the destructive step that usually precedes import
- [`forgeplan scan-import`](/docs/cli/scan-import/) — rebuild from markdown (preferred for fresh clones)
- [`forgeplan migrate`](/docs/cli/migrate/) — non-destructive alternative when only the schema has drifted
- [`forgeplan health`](/docs/cli/health/) — post-import verification
