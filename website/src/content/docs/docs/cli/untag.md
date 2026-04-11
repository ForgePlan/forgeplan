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
4. Rewrites frontmatter and re-indexes the artifact in LanceDB.

Unknown tags are silently ignored — `untag` is idempotent and safe to script.

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
forgeplan list --tag wip --format ids | \
  xargs -I {} forgeplan untag {} wip
```

For tag canonicalization across the whole workspace (PROB-026 fix), prefer
[`forgeplan reindex`](/docs/cli/reindex/) — it normalizes every tag in place
without needing explicit untag/tag pairs.

## Notes

- `untag` is case-insensitive on the input: `forgeplan untag PRD-001 Legacy`
  will remove `legacy`.
- If you want to move an artifact from one tag to another, run `untag` then
  `tag` — there is no `retag` helper.
- Removing a tag does not affect relations or scoring — tags are pure metadata.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan tag`](/docs/cli/tag/) — add tags
- [`forgeplan reindex`](/docs/cli/reindex/) — canonicalize tags workspace-wide
- [`forgeplan list`](/docs/cli/list/) — filter artifacts by tag
- [Methodology guide](/docs/methodology/overview/)
