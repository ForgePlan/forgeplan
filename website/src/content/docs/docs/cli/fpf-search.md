---
title: forgeplan fpf search
description: "Search the FPF knowledge base using the same BM25 + semantic pipeline as artifact search"
---

`forgeplan fpf search` queries the ingested **First Principles Framework** knowledge base using the same hybrid retrieval pipeline that powers artifact search — production BM25 (via the `bm25` crate v2.3.2) fused with BGE-M3 semantic vectors. It searches across all 204 FPF sections and returns ranked matches with section IDs and previews.

Think of it as `forgeplan search` pointed at the FPF corpus instead of your own artifacts.

## When to use

- **While shaping a PRD or ADR** — pull first-principles context on a concept before writing (`"trust calculus"`, `"bounded context"`, `"explore exploit"`).
- **While auditing a decision** — verify the framework's stance on a pattern you're considering.
- **While onboarding** — treat the KB as a searchable textbook.
- **While writing `forgeplan reason --fpf` prompts** — preview what the model will see as FPF context.

## When NOT to use

- For artifact search — use [`forgeplan search`](/docs/cli/search/) instead.
- For reading a whole section — use [`forgeplan fpf section <id>`](/docs/cli/fpf-section/).
- For browsing the full index — use [`forgeplan fpf list`](/docs/cli/fpf-list/).

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
# Default: BM25 keyword search, top 5 results
forgeplan fpf search "trust calculus"

# Widen the result window
forgeplan fpf search "explore exploit tradeoff" --limit 10

# Semantic vector search (falls back to BM25 if semantic-search feature is off)
forgeplan fpf search "adversarial review" --semantic

forgeplan fpf search "weakest link"
```

## How the pipeline works

The FPF search uses the **same v0.18.0 production pipeline** as artifact search:

- **BM25** (`bm25` crate v2.3.2) — tokenization with Russian morphology (Snowball stemmer) when `LanguageMode::Detect` triggers.
- **Template noise stripping** — removes boilerplate headings before scoring, so queries match substantive content.
- **Semantic fusion** — BGE-M3 cosine similarity layered on top when the `semantic-search` feature is enabled.
- **O(N) batch search** — efficient across the 204-section corpus.

Because the mechanics are identical, the same tuning and troubleshooting notes from the [search v2 guide](/docs/guides/search-v2/) apply here.

## How it fits

`fpf search` is the query surface of the FPF KB. It's consumed both by humans (CLI inspection) and by `forgeplan reason --fpf`, which retrieves top-k sections as context for the ADI prompt.

```
ingest → search → (human reads OR reason consumes) → better decisions
```

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) — parent command
- [`forgeplan fpf section`](/docs/cli/fpf-section/) — read a specific result
- [`forgeplan fpf list`](/docs/cli/fpf-list/) — browse all sections
- [`forgeplan search`](/docs/cli/search/) — artifact search (same pipeline)
- [Search v2 guide](/docs/guides/search-v2/) — how BM25 + semantic fusion work
- [`forgeplan reason`](/docs/cli/reason/) — ADI with FPF context
