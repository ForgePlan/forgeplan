---
title: forgeplan fpf search
description: "Search ingested FPF sections with weighted keyword matching or optional BGE-M3 vector retrieval"
---

`forgeplan fpf search` searches the ingested **First Principles Framework** sections and prints section IDs, titles, previews, and line counts. It searches the separate KB, not project artifacts. The corpus size depends on what you ingested.

By default it uses weighted case-insensitive substring matching. `--semantic` selects BGE-M3 vector search instead; these are separate retrieval paths, not the artifact search BM25/hybrid pipeline. Neither path calls an LLM provider.

## When to use

- **While shaping a PRD or ADR** - pull first-principles context on a concept before writing (`"trust calculus"`, `"bounded context"`, `"explore exploit"`).
- **While auditing a decision** - verify the framework's stance on a pattern you're considering.
- **While onboarding** - treat the KB as a searchable textbook.
- **Before `forgeplan reason <ID> --fpf`** - inspect possible KB context. Reason selects keywords from the artifact title itself, so a manual query does not reproduce its exact context.

## When NOT to use

- For artifact search - use [`forgeplan search`](/docs/cli/search/) instead.
- For reading a whole section - use [`forgeplan fpf section <id>`](/docs/cli/fpf-section/).
- For browsing the full index - use [`forgeplan fpf list`](/docs/cli/fpf-list/).

## Usage

```text
forgeplan fpf search [OPTIONS] <QUERY>
```

## Arguments

```text
  <QUERY>  Search query
```

## Options

```text
      --limit <LIMIT>  Max results [default: 5]
      --semantic       Use semantic vector search (requires --features semantic-search; falls back to keyword otherwise)
  -h, --help           Print help
  -V, --version        Print version
```

## Examples

```bash
# Default: weighted keyword search, top 5 results
forgeplan fpf search "trust calculus"

# Widen the result window
forgeplan fpf search "explore exploit tradeoff" --limit 10

# Semantic retrieval (warns and falls back to keyword on propagated runtime errors)
forgeplan fpf search "adversarial review" --semantic

forgeplan fpf search "weakest link"
```

## How the pipeline works

- **Keyword path:** splits the trimmed query on whitespace, keeps words longer than two bytes, and matches any word in title or body. Title matches add 50, body occurrences add up to 20 per word, all words in the title add 100, and a full-query title match adds 200. If no words survive, it matches the whole query as a substring. There is no stemming or BM25 scoring.
- **Semantic path:** encodes the query with BGE-M3 and asks LanceDB for nearest sections by cosine distance. It needs a semantic-enabled build, cached/downloadable model weights, and section vectors from ingest. Successful retrieval prints `[semantic search: BGE-M3]`.
- **Fallback:** unavailable build support, model initialization/encoding errors, or propagated vector errors produce a stderr warning and keyword retrieval. Some LanceDB vector failures are converted to an empty result by the store, so an empty semantic result does not always trigger keyword fallback.
- **Input and output:** blank queries are rejected; the length cap is 8192 bytes (the error says “chars”). Results are limited by `--limit` (default 5). Each preview uses up to three body lines, capped at 200 characters, followed by the section's line count. There is no JSON output flag or numeric score in the CLI listing.

If no sections match, inspect `fpf status` and `fpf list` before re-ingesting. An empty result can mean no match, an empty KB, or unavailable vectors. The [artifact search guide](/docs/guides/search-v2/) describes a different pipeline.

## How it fits

`fpf search` is the human-facing KB query command. `forgeplan reason <ID> --fpf` calls the keyword store directly using significant title words, deduplicates up to three sections, and includes the first 300 characters of each in its ADI prompt.

```
ingest → search → (human reads OR reason consumes) → better decisions
```

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf section`](/docs/cli/fpf-section/) - read a specific result
- [`forgeplan fpf list`](/docs/cli/fpf-list/) - browse all sections
- [`forgeplan search`](/docs/cli/search/) - project artifact search (different pipeline)
- [Search v2 guide](/docs/guides/search-v2/) - artifact BM25 and semantic retrieval
- [`forgeplan reason`](/docs/cli/reason/) - ADI with FPF context
