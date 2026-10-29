---
title: forgeplan fpf search
description: "Search ingested FPF sections by keyword or cached BGE-M3 vectors."
---

`forgeplan fpf search` queries the ingested **First Principles Framework** knowledge base. Default search uses case-insensitive title/body keyword matching. `--semantic` selects BGE-M3 vector search; it is not fused with keyword results. The corpus size depends on the sections you ingested, not a fixed count of 204.

## When to use

- **While shaping a PRD or ADR** - pull first-principles context on a concept before writing (`"trust calculus"`, `"bounded context"`, `"explore exploit"`).
- **While auditing a decision** - verify the framework's stance on a pattern you're considering.
- **While onboarding** - treat the KB as a searchable textbook.
- **While writing `forgeplan reason --fpf` prompts** - preview what the model will see as FPF context.

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
      --semantic       Use semantic vector search (requires --features semantic-search;
                       falls back to keyword otherwise)
  -h, --help           Print help
  -V, --version        Print version
```

## Examples

```bash
# Default: keyword search, up to 5 results
forgeplan fpf search "trust calculus"

# Widen the result window
forgeplan fpf search "explore exploit tradeoff" --limit 10

# Vector search; warns and falls back to keyword if unavailable
forgeplan fpf search "adversarial review" --semantic

forgeplan fpf search "weakest link"
```

## How the pipeline works

The FPF retrieval paths differ from artifact BM25 search:

- **Keyword** - substring matches in titles and bodies, with extra weight for title hits and full-query title matches.
- **Semantic** - BGE-M3 vector lookup when explicitly requested, using a compatible semantic-search build, cached model, and ingested section vectors.
- **Fallback** - a semantic initialization, encoding, or vector-query failure prints a warning and runs keyword search.

This path does not apply artifact BM25 stemming, template stripping, or semantic fusion. The [search v2 guide](/docs/guides/search-v2/) describes the separate artifact pipeline.

## How it fits

`fpf search` is the KB inspection surface. `reason --fpf` has its own context-loading path and, in the selected build, omits legacy FPF enrichment on a migrated workspace rather than reopening retired tables.

```
ingest → search → (human reads OR reason consumes) → better decisions
```

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf section`](/docs/cli/fpf-section/) - read a specific result
- [`forgeplan fpf list`](/docs/cli/fpf-list/) - browse all sections
- [`forgeplan search`](/docs/cli/search/) - separate artifact retrieval pipeline
- [Search v2 guide](/docs/guides/search-v2/) - artifact BM25 and semantic retrieval
- [`forgeplan reason`](/docs/cli/reason/) - ADI with FPF context

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf search --help`.

```text
Search FPF knowledge base

Usage: forgeplan fpf search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  Search query

Options:
      --limit <LIMIT>  Max results [default: 5]
      --semantic       Use semantic vector search (requires --features semantic-search; falls back to keyword otherwise)
  -h, --help           Print help
  -V, --version        Print version
```
<!-- forgeplan-runtime:end -->
