---
title: forgeplan delete
description: "Soft-delete an artifact with a recovery receipt."
---

Remove an artifact from the workspace after explicit `--yes` confirmation.
The command records a soft-delete receipt before removing canonical Markdown
and associated derived relations. Recovery through `forgeplan restore <ID>`
or `forgeplan undo-last` is normally available within the receipt's 30-day TTL,
subject to identity conflicts and surviving relation targets.

Deletion still changes history visible in the workspace. Prefer
[`forgeplan deprecate`](/docs/cli/deprecate/) or
[`forgeplan supersede`](/docs/cli/supersede/) for decisions being retired or replaced.

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

1. Refuses without `--yes`; this CLI does not prompt interactively.
2. Warns about incoming dependents; `--yes` also confirms proceeding despite them.
3. Captures the artifact and relations in `.forgeplan/trash/`.
4. Removes the canonical file and its derived row/relations. On Constellation,
   publication is reconciled after the file mutation and can remain pending.
5. Leaves other artifact bodies intact; their textual references may need review.

## Examples

Inspect the refusal before explicitly authorizing deletion:

```bash
forgeplan delete NOTE-042
```

Scripted delete (no prompt):

```bash
forgeplan delete NOTE-042 --yes
```

## Safety: when NOT to delete

Do **not** `delete` an active artifact as a way to "cancel" it. The methodology
treats abandoned decisions as first-class history - they remain discoverable and
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

## Before you delete - back up

The receipt supports recovery, but it is not a substitute for a backup.
Before bulk-deleting, run:

```bash
forgeplan export --output backup-$(date +%Y%m%d).json
```

And for peace of mind:

```bash
cp -r .forgeplan .forgeplan-backup-$(date +%Y%m%d)
```

Use `forgeplan restore NOTE-042` for a mistaken deletion. Keep the export and
workspace backup for recovery beyond receipt limits. See the
[agent setup guide](/docs/guides/claude-code-setup/) for workspace safety rules.

## Notes

- Incoming dependents produce a warning and require `--yes` to proceed.
- Canonical reads reflect removal; derived publication can lag a successful mutation.
- Prefer the receipt-based restore command over manually recovering one file:
  it also attempts relation recovery and reports skipped targets.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan deprecate`](/docs/cli/deprecate/) - terminal status, preserves history
- [`forgeplan supersede`](/docs/cli/supersede/) - replace with a newer decision
- [`forgeplan export`](/docs/cli/export/) - back up before bulk deletes
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan delete --help`.

```text
Delete an artifact

Usage: forgeplan delete [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID

Options:
      --yes      Skip confirmation
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
