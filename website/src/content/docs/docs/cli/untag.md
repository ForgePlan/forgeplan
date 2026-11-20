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

1. Loads the artifact's frontmatter.
2. Canonicalizes each incoming tag the same way `tag` does (lowercased,
   trimmed) so `Legacy`, `legacy`, and `  legacy  ` all match the stored value.
3. Removes each match from the `tags:` list.
4. Rewrites canonical frontmatter and updates the owning index; Constellation requests reconciliation rather than reopening legacy storage.

Unknown tags are silently ignored - `untag` is idempotent and safe to script.

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

Review IDs first:

```bash
forgeplan list --tag wip --json | jq -r '.[].id'
# After reviewing that list, remove the tag from each chosen artifact:
forgeplan untag PRD-001 wip
```

`index` maintains derived state; it is not a guarantee of source-tag rewriting.

## Notes

- `untag` is case-insensitive on the input: `forgeplan untag PRD-001 Legacy`
  will remove `legacy`.
- If you want to move an artifact from one tag to another, run `untag` then
  `tag` - there is no `retag` helper.
- Removing a provenance tag does not remove the standalone body `source_tier` field used by evidence scoring. Tags and declared scoring fields are different data.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan tag`](/docs/cli/tag/) - add tags
- [`forgeplan index`](/docs/cli/index-command/) - maintain the Constellation derived index
- [`forgeplan list`](/docs/cli/list/) - filter artifacts by tag
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan untag --help`.

```text
Remove tags from an artifact

Usage: forgeplan untag <ID> <TAGS>...

Arguments:
  <ID>       Artifact ID
  <TAGS>...  Tags to remove

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
