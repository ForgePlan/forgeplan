---
title: forgeplan recall
description: "Search local Forgeplan core memory saved with remember. This is not Hindsight recall and does not search a remote bank. Memory records use kind memory in the workspace store."
---

Search local Forgeplan core memory saved with `remember`. This is not Hindsight recall and does not search a remote bank. Memory records use kind `memory` in the workspace store.

## Usage

```text
forgeplan recall [OPTIONS] [QUERY]
```

## Arguments

```text
  [QUERY]  Search query (substring match in title/body)
```

## Options

```text
  -c, --category <CATEGORY>  Filter by category
  -n, --limit <LIMIT>        Max results (default: 10) [default: 10]
      --json                 Output as JSON for machine consumption
  -h, --help                 Print help
  -V, --version              Print version
```

## Search and output

Query matching is case-insensitive substring search in title/body, not BM25 or semantic ranking. Category matching is case-insensitive; results are newest first and limited by `--limit` (default 10). JSON contains `memories` and `_next_action`; each memory has `id`, `category`, `author`, `created_at`, and `text`, not tags or an auto-retain source field.

## Examples

```bash
forgeplan recall "branch workflow"
forgeplan recall --category procedure
forgeplan recall --json --limit 1000 > memories.json
```

## Next steps

Use `search` for ordinary artifacts, `journal` for decisions, and `promote MEMORY_ID --kind note` to turn a chosen memory into a draft artifact. Copy the actual memory ID from the result. Promotion removes the original memory; it is not an in-place status change.

## See also

- [`forgeplan search`](/docs/cli/search/) - artifact search
- [`forgeplan journal`](/docs/cli/journal/) - decision timeline
- [`forgeplan promote`](/docs/cli/promote/) - promote a memory to a draft artifact
