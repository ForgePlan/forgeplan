---
title: Smart Search (v0.18 BM25 + Russian morphology)
description: "How Forgeplan's production BM25 engine finds artifacts - stemming, multi-language support, and CLI/MCP usage."
---

Forgeplan combines lexical retrieval with optional semantic vectors. The
v0.18 sections below explain the historical BM25 engine; benchmark numbers
are historical observations, not measurements of the local Constellation build.

For migrated workspaces in local `0.39.0+constellation.s1.1` (not published
0.38.0), see the [Constellation guide](/docs/guides/constellation/) and
[reference](/docs/reference/constellation/). Ordinary `search` defaults to
20 results; separate `constellation search` defaults to 10.

## Who this is for

- You wrote an artifact months ago and only remember "something about auth".
- You work in a bilingual team (English + Russian) and want both queries to find
  the same PRD.
- You build an AI agent on top of Forgeplan's MCP server and need deterministic,
  fast retrieval.

## Production BM25 engine (v0.18.0)

Before v0.18 Forgeplan shipped a 140-line hand-written BM25 implementation. It
worked, but it was hard to tune, had no built-in stemmer, and scored records
one at a time - O(N²) in practice once the workspace grew past a hundred
artifacts. v0.18.0 replaces that prototype with the production-grade
[`bm25` crate v2.3.2](https://crates.io/crates/bm25), which brings four
concrete wins:

- **Proper term weighting.** Production TF-IDF / BM25 with length
  normalization, instead of our naive substring scoring.
- **O(N) batch search.** A single `search_scores()` call ranks the whole
  corpus in one pass. A 193-artifact workspace returns in **0.23s**,
  down from the multi-second stalls that made `search` feel laggy.
- **Built-in tokenizer stack.** Unicode segmentation, stop-word removal,
  and (crucially) pluggable stemmers - which is what enables Russian
  morphology in the next section.
- **Template noise stripping.** `strip_indexing_noise()` runs before
  tokenization and drops YAML frontmatter, `{placeholder}` lines,
  `|table|` rows, and HTML comments from the indexed text. This fixes
  PROB-030: on v0.17, `forgeplan search "auth"` would match every PRD
  that had `author:` in its frontmatter, flooding the results with
  false positives. v0.18 indexes only real body content.

These four changes land together in v0.18.0 and are covered by regression
tests in `crates/forgeplan-core/src/search/` - see the CHANGELOG entry.

## How it works

In smart mode, lexical ranking and optional semantic scores are combined
before optional graph expansion. Forced keyword and semantic modes use
different paths.

1. **Lexical (BM25 + substring)** - BM25 ranks terms by frequency, inverse
   document frequency and length normalization; a substring channel also
   supports partial matches. No measured query-coverage percentage is implied.
2. **Filters and boosters** - kind/status filters restrict candidates.
   R_eff, active status and graph centrality boost matching results.
3. **Graph expansion** - results are optionally enriched with neighbors via
   typed links (`informs`, `refines`, `supersedes`) so a hit on a PRD also
   surfaces its RFC and evidence.

An optional semantic stage based on BGE-M3 embeddings can be layered on top -
see [Semantic search](#semantic-search-feature-flag) below.

### Tokenization and stemming

The document body is cleaned before indexing; queries use the same tokenizer
but do not pass through document noise stripping:

```
document body → strip_indexing_noise() → prepend title
document text or query
  → whichlang detect             // per document/query; detection can be wrong
  → lowercase + unicode split
  → stop-word filter
  → Snowball stem (when the detected language has a supported stemmer)
  → BM25 term vector
```

When both sides select the Russian stemmer, noun forms such as
`"аутентификация"` and `"аутентификации"` can share a stem and match.

### Template noise stripping

PRDs and RFCs created from Forgeplan templates contain a lot of structural
boilerplate that used to pollute the index:

- YAML frontmatter (`id:`, `author:`, `status:` …)
- Placeholder lines like `{problem statement}` left behind in stub artifacts
- Markdown tables (`| FR | Description | Status |`)
- HTML comments from template hints

Before v0.18 a query for `auth` would match the word `author:` in every single
frontmatter block. `strip_indexing_noise()` removes these sections before they
reach the tokenizer, so lexical scores reflect real content only. This is
tracked as PROB-030.

### O(N) batch search

The v0.17 implementation called `.score()` per record, giving O(N²) behaviour
on workspaces with hundreds of artifacts. v0.18 uses `search_scores()` from the
`bm25` crate, which ranks the whole corpus in a single pass:

- 193-artifact workspace: **0.23s** end-to-end
- This dated result is not a latency guarantee for another machine or corpus.

## Multi-language morphology

v0.18.0 enabled `LanguageMode::Detect`. The pinned `bm25` 2.3.2 tokenizer
has 17 fixed-language variants, but automatic detection maps only a subset
to stemmers: Arabic, German, English, French, Italian, Dutch, Portuguese,
Russian, Spanish, Swedish and Turkish. Unsupported detections use no
stemmer. This is not translation between languages.

Illustrative result shape for a Russian noun query, not a runtime transcript:

```
$ forgeplan search "аутентификация"
PRD-019 Auth middleware    (matches "аутентификации" via stem "аутентификац")
EVID-023 Auth benchmark    (matches "аутентификацию")
```

Inflected nouns can normalize to the same stem when language detection
agrees. Detection, corpus statistics and other ranking channels can affect
the result and score; equal scores are not guaranteed.

## Russian morphology in practice

Create a PRD in Russian:

```bash
forgeplan new prd "Система аутентификации"
```

Try noun inflections and a related verb separately:

```bash
forgeplan search "аутентификация"     # nominative
forgeplan search "аутентификации"     # genitive / dative
forgeplan search "аутентификацией"    # instrumental
forgeplan search "аутентифицировать"  # verb form
```

Stemming is not a synonym or derivation engine. Do not assume the verb
`аутентифицировать` shares the noun's stem, or that every English derivative
has one stem. Short or mixed-language queries can also be misclassified.
Compare actual results rather than requiring identical scores.

## CLI usage

All examples assume you are inside a Forgeplan workspace.

### 1. Simple query

```bash
forgeplan search "bm25 russian morphology"
```

Prints up to 20 hits by default with kind, ID, title and score.

### 2. Filter by kind

```bash
forgeplan search "auth" --type prd
forgeplan search "auth" --type rfc
forgeplan search "auth" --type evidence
```

Only artifacts of the requested kind are returned. Useful when you want the
RFC that explains how a PRD gets built.

### 3. Filter by status

```bash
forgeplan search "tags canonicalization" --status active
forgeplan search "deprecated semantics"   --status deprecated
```

Combine with `--type` to scope precisely:

```bash
forgeplan search "scoring" --type prd --status active
```

### 4. Limit results

```bash
forgeplan search "LanceDB" --limit 5
```

The default is 20. For scripting, set `--limit 1` to request one hit.

### 5. Reindex after manual edits

After an intentional canonical Markdown change, publish the derived index
with the command appropriate to its layout:

```bash
forgeplan index  # Constellation; use forgeplan reindex for the legacy layout
```

`reindex` reads canonical artifact Markdown. On a Constellation-owned layout
it routes through reconcile rather than creating legacy tables. A fresh
canonical read and a persisted index generation are different states.

## MCP usage

The MCP server exposes the same engine as a tool called `forgeplan_search`.
Agents can invoke it via stdio. Example JSON call:

```json
{
  "name": "forgeplan_search",
  "arguments": {
    "query": "semantic scoring intelligence",
    "kind": "prd",
    "status": "active",
    "limit": 5
  }
}
```

The MCP payload uses `results`, `total`, and `coverage` (not `hits` or
`took_ms`). See [`forgeplan_search`](/docs/mcp/forgeplan_search/) for fields.
This illustrative empty payload is a shape example, not a runtime transcript:

```json
{
  "results": [],
  "total": 0,
  "coverage": {
    "semantic": false,
    "lexical": true,
    "graph": false
  }
}
```

MCP uses `kind` while the CLI flag is `--type`. MCP also accepts `workspace`
and `no_expand`; it defaults to 20 results. `coverage` reports channel state,
not proof that each channel contributed to this response's ranking. In the
current full-MCP smart path, `coverage.graph` is set from `!no_expand` even
if graph loading failed and no graph reached the ranker. A true flag therefore
does not establish graph contribution or completeness; a false semantic flag
still identifies an incomplete negative without semantic retrieval.

## Semantic search (feature flag)

Semantic retrieval needs a binary built with `semantic-search`, the cached
BGE-M3 snapshot, and eligible vectors in the workspace index. The current
engine is tract, not the older fastembed runtime; only `bge-m3` is supported.
Model weights are roughly 2.1 GB, separate from the binary.

```bash
cargo install forgeplan --features semantic-search
forgeplan setup
forgeplan embed
forgeplan search "how do I prove a decision is sound" --semantic
```

The cache root is `FORGEPLAN_MODEL_CACHE` if set, otherwise
`~/.forgeplan/models` (platform/legacy fallback only without home).
`HF_HOME` does not override tract. Queries and maintenance do not download
weights. Background embedding also requires explicit `embedding:` config;
`--auto-index` alone and a prior manual `embed` do not configure it.

On a migrated workspace, forced `--semantic` does not guarantee keyword
fallback. Check availability and coverage. Use `--keyword` explicitly when
you need lexical retrieval without semantic prerequisites.

## Troubleshooting

**Score is always 0 or no results come back.**
Inspect the query, filters, source content and selected mode first. If the
persisted generation is stale, use `index` for Constellation or `reindex`
for the legacy layout. An empty result alone does not establish the cause.

**Query in Russian finds nothing but the same English term works.**
Check detected-language limitations and noun/verb differences above.
Morphology is not translation, and reindexing cannot make every related
word share a stem.

**Too many irrelevant hits from frontmatter fields.**
Check the selected version and the matching source fields. Noise stripping
does not prove that every irrelevant result comes from an old binary.

**Search is slow on a large workspace.**
The historical v0.18 record reports 0.23s for 193 artifacts; it is not a
sub-second service guarantee. Record the selected mode, version, corpus size
and observed latency when reporting a performance issue.

**I deleted a file but it still shows up in results.**
Inspect the source and persisted generation separately. Reconcile with
`index` for Constellation or `reindex` for the legacy layout; canonical
reads and persisted-index freshness are different contracts.

## Related artifacts

- **PRD-039** - Smart Search v2 (original v0.17 design)
- **PRD-040** - Scoring Intelligence (ranking signals feeding into search)
- **PRD-042** - FPF KB vector search (semantic search feature flag)
- **PROB-026** - Tag canonicalization (query-side normalization)
- **PROB-027** - Reindex without `lance/` folder
- **PROB-030** - `auth` prefix false positives from frontmatter
- **CHANGELOG v0.18.0** - production BM25 + Russian morphology release notes

## See also

- [`forgeplan search` CLI reference](/docs/cli/search/)
- [`forgeplan_search` MCP tool reference](/docs/mcp/forgeplan_search/)
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Ten rules of Forgeplan methodology](/docs/guides/ten-rules/)
