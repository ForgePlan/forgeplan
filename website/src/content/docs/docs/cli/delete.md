---
title: forgeplan delete
description: "Remove an artifact and its incident relations from the live workspace. In 0.38.0 deletion is soft: a receipt and saved projection in .forgeplan/trash/ support recovery within the retention window (30 days by default)."
---

Remove an artifact and its incident relations from the live workspace. In 0.38.0 deletion is soft: a receipt and saved projection in `.forgeplan/trash/` support recovery within the retention window (30 days by default).

## Usage

```text
forgeplan delete [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID
```

## Options

```text
      --yes      Skip confirmation
  -h, --help     Print help
  -V, --version  Print version
```

## Confirmation and changes

CLI deletion is non-interactive: without `--yes` it refuses and prints a confirmation hint; it does not ask a question. With `--yes`, it captures a recovery receipt, removes the live artifact and incident relations, and moves the projection to trash. Incoming dependents are warned about, not protected from an explicitly confirmed deletion. The captured help’s “Skip confirmation” does not mean that an interactive prompt exists.

## Examples

```bash
# Inspect the artifact and its links before confirming deletion:
forgeplan get NOTE-043
forgeplan delete NOTE-043 --yes
# Recover while its receipt remains available:
forgeplan restore NOTE-043
# Alternatively, undo the latest supported destructive operation:
# forgeplan undo-last
```

## Preserve decision history

Use `supersede` for replacement and `deprecate` for a decision no longer relevant. Both preserve discoverable history without relying on trash. Delete mistaken or duplicate stubs, not the explanation of why an active decision was abandoned.

## Recovery and backup

`restore` refuses a conflicting live ID and restores relations only where endpoints still exist. Recovery requires an unconsumed retained receipt and is not a backup policy. Export before bulk deletion and keep necessary filesystem backups. Returning markdown from Git and running `reindex` does not recreate relations missing from that markdown.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan deprecate`](/docs/cli/deprecate/) - terminal status, preserves history
- [`forgeplan supersede`](/docs/cli/supersede/) - replace with a newer decision
- [`forgeplan export`](/docs/cli/export/) - back up before bulk deletes
- [Methodology guide](/docs/methodology/overview/)
