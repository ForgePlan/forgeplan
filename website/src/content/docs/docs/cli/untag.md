---
title: forgeplan untag
description: "Remove tags from an artifact"
---

Remove one or more tags from an artifact. The inverse of
[`forgeplan tag`](/docs/cli/tag/). Use this to prune outdated labels (`legacy`
that is no longer legacy, `wip` on something now shipped) or to fix mistakes
from a mass-tagging operation.

## Usage

```text
forgeplan untag <ID> <TAGS>...
```

## Arguments

```text
  <ID>       Artifact ID
  <TAGS>...  Tags to remove
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## What it does

1. Loads the stored record by exact ID and synchronizes existing projection edits.
2. Removes tags by **exact string equality**. Unlike `tag`, it does not trim or lowercase incoming values.
3. Updates the index and renders the Markdown projection.

Use the canonical stored spelling (`legacy`, not `Legacy`) and exact record ID from `list`. Unknown tags are ignored. Text output reports the number of supplied arguments as “Removed”, not the number actually removed; inspect `Current tags` for the resulting state. There is no JSON mode.

## Examples

Remove a single tag:

```bash
forgeplan untag PRD-001 legacy
```

Remove multiple tags in one call:

```bash
forgeplan untag PRD-018 wip experimental source=draft
```

Fix a mass-tag mistake (tag then immediately untag):

```bash
forgeplan tag PRD-001 securiti   # typo
forgeplan untag PRD-001 securiti
forgeplan tag PRD-001 security   # correct
```

## Batch cleanup

To strip a tag across every artifact (e.g. retiring a taxonomy), combine with
`list --tag`:

```bash
forgeplan list --tag wip --json | jq -r '.[].id' | \
  xargs -I {} forgeplan untag {} wip
```

`reindex` imports projection tags when creating a missing record, but its incremental synchronization of an existing record updates the body, not the stored tags column. It is neither an existing-tag metadata repair nor a workspace-wide in-place tag normalizer. Correct existing metadata through `untag` / `tag`; for legacy mixed-case labels, remove the exact stored spelling before adding the desired canonical value.

## Notes

- `forgeplan untag PRD-001 Legacy` does not remove a canonical stored `legacy`; use the lowercase exact value.
- If you want to move an artifact from one tag to another, run `untag` then
  `tag` - there is no `retag` helper.
- Removing a source-tier tag does not change scoring CL in 0.38.0; CL comes from evidence body fields. Relations are unchanged.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan tag`](/docs/cli/tag/) - add tags
- [`forgeplan reindex`](/docs/cli/reindex/) - synchronize existing bodies and index missing projection records
- [`forgeplan list`](/docs/cli/list/) - filter artifacts by tag
- [Methodology guide](/docs/methodology/overview/)
