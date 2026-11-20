---
title: forgeplan recall
description: "Search and filter your saved memories (not artifacts)"
---

Search the **memory store** - ad-hoc observations, lessons, and context
snippets saved via `forgeplan remember`. This is Forgeplan's lightweight
personal note-taking lane, separate from artifacts. Use `recall` when you
remember "we learned something about X" but there is no formal PRD or Note.

## When to use

- "We agreed not to use crate X - why?"
- Restore context at session start ("recall 'sprint 13'")
- Pull prior lessons before a retro or postmortem
- Fuel an AI agent with situational memory

## Not to use when

- You want artifacts (PRD, RFC, ADR) → use [`forgeplan search`](/docs/cli/search/)
- You want decisions → use [`forgeplan journal`](/docs/cli/journal/)

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

## Examples

Recall everything matching a phrase:

```bash
forgeplan recall "branch workflow"
```

All memories in a category:

```bash
forgeplan recall --category build
```

Write up to 1000 matching records as JSON (not an exhaustive export):

```bash
forgeplan recall --json --limit 1000 > memories.json
```

## Output interpretation

Illustrative memory content, not a project policy or runtime transcript:

```
[2026-04-05]  build    Do not delete feature branches after merge
   PRs to dev are merge-commit, squash loses late commits. Keep the
   branch until the PR is closed for at least a week.
```

Text includes ID, category, author, date and an 80-character excerpt.
JSON wraps records in `memories` with `id`, `category`, `author`, `created_at`
and `text`; it does not supply arbitrary tags or a manual/auto source field.

Unlike [`search`](/docs/cli/search/), `recall` is pure substring match with
category filtering - fast and predictable, no BM25 or semantic ranking.

This is Forgeplan's local memory, distinct from an external Hindsight service.

## How it fits

Forgeplan separates **artifacts** (decisions with lifecycle) from **memories**
(ad-hoc notes without lifecycle):

```
artifacts   →  list, get, search, graph
memories    →  remember, recall
```

If a memory is referenced multiple times, promote it to a `Note` artifact and
link it - that's when it earns lifecycle and evidence.

## See also

- [`forgeplan search`](/docs/cli/search/) - artifact search
- [`forgeplan journal`](/docs/cli/journal/) - decision timeline
- [`forgeplan new note`](/docs/cli/new/) - promote a memory to a Note

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan recall --help`.

```text
Recall memories — search, filter, list

Usage: forgeplan recall [OPTIONS] [QUERY]

Arguments:
  [QUERY]  Search query (substring match in title/body)

Options:
  -c, --category <CATEGORY>  Filter by category
  -n, --limit <LIMIT>        Max results (default: 10) [default: 10]
      --json                 Output as JSON for machine consumption
  -h, --help                 Print help
  -V, --version              Print version
```
<!-- forgeplan-runtime:end -->
