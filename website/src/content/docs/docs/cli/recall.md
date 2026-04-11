---
title: forgeplan recall
description: "Search and filter your saved memories (not artifacts)"
---

Search the **memory store** — ad-hoc observations, lessons, and context
snippets saved via `forgeplan remember`. This is Forgeplan's lightweight
personal note-taking lane, separate from artifacts. Use `recall` when you
remember "we learned something about X" but there is no formal PRD or Note.

## When to use

- "We agreed not to use crate X — why?"
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

Dump everything as JSON:

```bash
forgeplan recall --json --limit 1000 > memories.json
```

## Output interpretation

Each hit shows a short card:

```
[2026-04-05]  build    Do not delete feature branches after merge
   PRs to dev are merge-commit, squash loses late commits. Keep the
   branch until the PR is closed for at least a week.
```

Columns: date, category, title, then a body excerpt. `--json` returns the
full record including any tags and the source (manual vs auto-retain).

Unlike [`search`](/docs/cli/search/), `recall` is pure substring match with
category filtering — fast and predictable, no BM25 or semantic ranking.

## How it fits

Forgeplan separates **artifacts** (decisions with lifecycle) from **memories**
(ad-hoc notes without lifecycle):

```
artifacts   →  list, get, search, graph
memories    →  remember, recall
```

If a memory is referenced multiple times, promote it to a `Note` artifact and
link it — that's when it earns lifecycle and evidence.

## See also

- [`forgeplan search`](/docs/cli/search/) — artifact search
- [`forgeplan journal`](/docs/cli/journal/) — decision timeline
- [`forgeplan new note`](/docs/cli/new/) — promote a memory to a Note
