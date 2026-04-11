---
title: forgeplan delete
description: "Permanently delete an artifact"
---

Permanently remove an artifact from the workspace. This deletes the markdown
file on disk and cascades through the LanceDB index, dropping every relation
where the artifact appears as source or target.

**Deletion is destructive and not reversible.** Unless you are cleaning up a
true mistake (e.g. a test artifact, a typo'd ID, a duplicate), you almost
certainly want [`forgeplan deprecate`](/docs/cli/deprecate/) or
[`forgeplan supersede`](/docs/cli/supersede/) instead — both preserve decision
history, which is the whole point of the methodology.

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

## What it does

1. Confirms the deletion interactively (skip with `--yes`).
2. Removes the markdown file from `.forgeplan/<kind>s/<id>.md`.
3. Deletes the artifact row from the LanceDB `artifacts` table.
4. Cascade-deletes every relation in the `links` table where the artifact is
   source or target.
5. Leaves other artifacts intact — but they may now reference a deleted ID.

## Examples

Delete a stray Note after interactive confirmation:

```bash
forgeplan delete NOTE-042
```

Scripted delete (no prompt):

```bash
forgeplan delete NOTE-042 --yes
```

## Safety: when NOT to delete

Do **not** `delete` an active artifact as a way to "cancel" it. The methodology
treats abandoned decisions as first-class history — they remain discoverable and
explain why the project went a different way.

| Situation                                       | Do this instead                                                  |
|-------------------------------------------------|------------------------------------------------------------------|
| Decision was superseded by a newer one          | [`forgeplan supersede <old> --by <new>`](/docs/cli/supersede/)   |
| Decision is no longer relevant                  | [`forgeplan deprecate <id> --reason "..."`](/docs/cli/deprecate/)|
| Artifact is an orphan with no content yet       | Delete is fine                                                   |
| You created a typo'd ID (`PRD-0001` instead of `PRD-1`) | Delete is fine                                           |
| Experimental Note that was never linked         | Delete is fine                                                   |

Rule of thumb: if anything links **to** the artifact, deprecate. If it's a
dead-end stub with no history, delete.

## Before you delete — back up

Cascade deletion of relations cannot be undone without a git restore or an
earlier export. Before bulk-deleting, run:

```bash
forgeplan export --output backup-$(date +%Y%m%d).json
```

And for peace of mind:

```bash
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
```

You can then `forgeplan import backup.json` to restore if you regret the
operation. See the [AGENT-ENFORCEMENT](/docs/guides/claude-code-setup/)
guide for why `rm -rf .forgeplan` is a banned shortcut.

## Notes

- `delete` does **not** check whether other artifacts reference the target. It
  is the caller's responsibility to ensure no active decision depends on it.
- Deleted artifacts disappear from `forgeplan health`, `forgeplan list`, and
  search results immediately.
- If you deleted by mistake and have not committed yet, `git checkout
  .forgeplan/<kind>s/<id>.md` followed by `forgeplan scan-import` will restore
  the file and re-index it.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan deprecate`](/docs/cli/deprecate/) — terminal status, preserves history
- [`forgeplan supersede`](/docs/cli/supersede/) — replace with a newer decision
- [`forgeplan export`](/docs/cli/export/) — back up before bulk deletes
- [Methodology guide](/docs/methodology/overview/)
