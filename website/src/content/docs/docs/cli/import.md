---
title: forgeplan import
description: "Restore artifacts from a JSON export file - reinit recovery, backups, and workspace migration"
---

`forgeplan import` reads a `forgeplan export` JSON bundle and writes canonical Markdown artifacts and declared relations into the current workspace, then updates the owning derived index. It is not a byte-for-byte workspace restore and does not recover all scoring history, configuration, receipts or session state. Keep a complete workspace backup as well as the JSON export.

## When to use

- Recovering supported artifact fields and declared relations into an isolated workspace while retaining the original backup.
- Transferring an export bundle between machines after checking version compatibility.
- Inspecting a pre-migration export without deleting the existing workspace.
- Cloning a teammate's workspace state for debugging or reproducing a bug.

## When NOT to use

- To merge blindly into a live workspace — existing identities are skipped without `--force`; force replaces them and can destroy current content.
- To rebuild an index from canonical Markdown — use `forgeplan index` for Constellation or the corresponding legacy `forgeplan reindex`, not `scan-import`.
- To import data from another tool - Forgeplan's JSON schema is internal. Only files written by `forgeplan export` are supported.

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
mkdir recovery-workspace
cd recovery-workspace
forgeplan init -y
forgeplan import ../backup.json
forgeplan health
```

An isolated recovery inspection, not a full disaster-recovery guarantee. Compare canonical bodies and relations with the retained workspace backup before using the result.

### Example 2: Force-overwrite on conflict

```bash
forgeplan import backup.json --force
```

Without `--force`, existing artifact identities are skipped. Active stubs can be downgraded to draft; with force that stub protection is bypassed. Relation application can be partial, so inspect the summary and warnings. Use force only after comparing the backup with current canonical content.

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

Useful for transferring the bundle's supported artifact fields and declared relations. It is not a backup of scoring history, decay history or all workspace state.

## How it fits the workflow

`import` is a recovery / migration tool, not part of the daily Shape → Validate → Code → Evidence → Activate cycle. It pairs tightly with `export`:

1. Export and retain a complete workspace backup before destructive operations.
2. Inspect the bundle by importing into a separate initialized workspace.
3. Compare bodies, statuses, counts and relations; health alone does not prove equivalence.

For routine canonical-index maintenance, use `forgeplan index` on Constellation.
`scan-import` discovers foreign documents; it is not recovery of existing canonical files.

## Safety notes

- **Run `forgeplan health` after import.** Compare supported artifact fields and relations against the retained backup. Fresh scores may differ; the JSON bundle is not a scoring-history restore.
- **`--force` is destructive.** It replaces existing canonical content. If in doubt, inspect the import in a separate initialized workspace instead.
- **The bundle format is version-dependent.** Check compatibility in isolation; do not assume `migrate` converts arbitrary old export bundles.
- **Do not edit export JSON casually.** Invalid fields or relations can change the imported result; there is no documented export-checksum validation guarantee here.
- **`import` writes Markdown.** Force can remove and recreate a conflicting canonical artifact. On Constellation, publication follows the file-first writes; inspect reconciliation diagnostics.

## See also

- [`forgeplan export`](/docs/cli/export/) - the other half of the backup pair
- [`forgeplan init`](/docs/cli/init/) - initialize a separate recovery target
- [`forgeplan scan-import`](/docs/cli/scan-import/) - import foreign Markdown, not rebuild canonical state
- [`forgeplan migrate`](/docs/cli/migrate/) - non-destructive alternative when only the schema has drifted
- [`forgeplan health`](/docs/cli/health/) - post-import verification

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan import --help`.

```text
Import artifacts from JSON file

Usage: forgeplan import [OPTIONS] <PATH>

Arguments:
  <PATH>  Path to JSON export file

Options:
      --force    Overwrite existing artifacts
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
