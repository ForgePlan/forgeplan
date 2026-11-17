---
title: Smart Search (BM25, morphology, and semantic search)
description: "Search artifacts with Forgeplan 0.38: lexical ranking, BGE-M3 prerequisites, filters, index recovery, and CLI/MCP differences."
---

Forgeplan 0.38 combines lexical ranking, optional semantic similarity, and graph
signals. BM25 matches terms and word forms; it does not translate a query or
understand synonyms by itself. BGE-M3 embeddings supply the semantic channel.
The BM25 changes introduced in v0.18 are described below as release history;
the command examples and prerequisites apply to 0.38.

## Who this is for

- You wrote an artifact months ago and only remember "something about auth".
- You work in a bilingual team (English + Russian) and need morphology-aware
  lexical matching, with optional cross-language semantic retrieval.
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

These changes shipped together in v0.18.0. The timing above is a historical
measurement, not a latency guarantee for your workspace.

## How it works

Smart search combines these signals:

1. **Lexical matching** — BM25 and substring scores rank title/body matches.
2. **Semantic similarity** — CLI smart search adds vector scores when available.
3. **Boosters and filters** — exact IDs, R_eff, recency, kind, status, depth,
   evidence availability, and creation date help rank or narrow results.
4. **Graph expansion** — one-hop neighbors can appear alongside direct matches.
   Use `--no-expand` for strict matches; this also removes the centrality booster.

An optional semantic stage based on BGE-M3 embeddings can be layered on top -
see [Semantic search](#semantic-search-feature-flag) below.

### Tokenization and stemming

Artifact bodies are cleaned first; titles and cleaned bodies form the corpus.
Documents and queries then use the same language-aware tokenizer:

```
title + cleaned body (or query text)
  → whichlang detect             // per-document and per-query
  → lowercase + unicode split
  → Snowball stem (per detected language)
  → stop-word filter
  → BM25 term vector
```

Because both sides use the same stemmer, `"аутентификация"` in a PRD body and
`"аутентификации"` in your query collapse to the same stem and match.

### Template noise stripping

PRDs and RFCs created from Forgeplan templates contain a lot of structural
boilerplate that used to pollute the index:

- YAML frontmatter (`id:`, `author:`, `status:` …)
- Placeholder lines like `{problem statement}` left behind in stub artifacts
- Markdown tables (`| FR | Description | Status |`)
- Single-line HTML comments (`<!-- ... -->`) from template hints

Before v0.18 a query for `auth` would match the word `author:` in every single
frontmatter block. `strip_indexing_noise()` removes these sections before they
reach the tokenizer, so lexical scores reflect real content only. This is
tracked as PROB-030.

The stripping is mechanical: real Markdown table rows are removed too, while
multi-line HTML comment blocks are not fully removed by this helper. Put
important searchable details in prose as well; do not assume every body field
contributes to BM25.

### O(N) batch search

The v0.17 implementation called `.score()` per record, giving O(N²) behaviour
on workspaces with hundreds of artifacts. v0.18 uses `search_scores()` from the
`bm25` crate, which ranks the whole corpus in a single pass:

- Historical v0.18 measurement: **0.23s** end-to-end for 193 artifacts.
- Actual latency depends on corpus size, graph work, model startup, and hardware.

## Multi-language morphology

v0.18.0 enables `LanguageMode::Detect` in the BM25 tokenizer: each document
and each query is inspected by [`whichlang`](https://crates.io/crates/whichlang),
which picks the right Snowball stemmer on the fly. Seventeen languages are
supported out of the box - English, Russian, German, French, Spanish,
Italian, Portuguese, Dutch, Swedish, Norwegian, Danish, Finnish, Hungarian,
Romanian, Turkish, Arabic, and a generic fallback.

For example, a body containing `"аутентификации"` can match the query
`"аутентификация"` through stemming. This is an illustration, not recorded CLI
output. Language detection and surrounding text affect tokenization; identical
scores across different queries are not guaranteed.

## Russian morphology in practice

Create a Russian artifact and fill its body with relevant content:

```bash
forgeplan new prd "Система аутентификации"
# Save the completed body in auth-prd-body.md, then use the returned ID:
forgeplan update PRD-001 --body @auth-prd-body.md
forgeplan search "аутентификация" --no-expand
forgeplan search "аутентификации" --no-expand
forgeplan search "аутентификацией" --no-expand
```

The IDs in examples assume a fresh workspace; use the ID returned by `new`.
Inflections can share a stem, but a verb such as `аутентифицировать` is not
guaranteed to share the noun's stem. The same caution applies to English
derivations. Stemming is lexical normalization, not a synonym engine.

## CLI usage

All examples assume you are inside a Forgeplan workspace.

### 1. Simple query

```bash
forgeplan search "bm25 russian morphology"
```

Prints up to 20 hits by default, with kind, ID, title, status, and score.

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

The smart-search default is 20. Set `--limit 1` for one ranked hit; it is not a confidence guarantee.

### 5. Reindex after manual edits

Prefer `forgeplan update ID --body @body.md` or the MCP update tool for artifact
edits. If files changed outside those tools, rebuild the derived LanceDB index:

```bash
forgeplan reindex
```

`reindex` reads artifact markdown, synchronizes bodies and frontmatter links,
recreates missing index tables, and removes orphan rows/relations. It is not a
persisted BM25 term-table rebuild: BM25 ranks the indexed records at query time.
Review reported errors and duplicate IDs rather than treating completion as a
guarantee that every file synchronized. Metadata-only edits are not a reliable
sync path for existing records; use lifecycle/update commands.

`scan-import` is different: it discovers documents to import as new artifacts.
It does **not** repair existing indexed bodies. Neither command generates
embeddings; after body changes, run `forgeplan embed` if you need semantic search.

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

The MCP result contains `results`, `total`, and `_next_action`; each result
includes ranking fields such as `score`, `bm25_score`, `semantic_score`, `r_eff`,
and `expanded_from`, plus artifact identity fields. It does not return the
`hits`/`took_ms` shape sometimes shown in older examples.

The default limit is 20. Other arguments include `depth`, `with_evidence`,
`no_evidence`, `since`, `no_expand`, `mode`, and `workspace` (an explicit
workspace path for a worktree).

:::caution[CLI/MCP differences in 0.38]
MCP `forgeplan_search` accepts `mode: "semantic"`, but its smart/semantic handler
does not supply vector scores. Do not use it as proof of semantic retrieval;
use CLI `search --semantic` for that. MCP FPF search is a separate tool.

CLI `--keyword` is substring search, not BM25. In 0.38, the forced keyword and
semantic CLI paths use the kind filter but not the full smart-search
filter/limit pipeline; semantic-only returns at most 10 hits. Use default smart
mode for combined filters and the documented `--limit`.
:::

## Semantic search (feature flag)

Published 0.38 release binaries include `semantic-search`; a source build needs
`--features semantic-search`. Inference uses BGE-M3 with the pure-Rust `tract`
engine. The model is a separate **~2.1 GB download**, not ~400 MB added to the
binary. Only `embedding.model: bge-m3` is supported.

Prepare the model and index deliberately:

```bash
forgeplan setup              # model, fpl alias, and git hooks
forgeplan embed              # encode missing/changed artifact bodies
forgeplan search "how do I prove a decision is sound"
forgeplan search "how do I prove a decision is sound" --semantic --json
```

`init -y` does not download the model; `init --with-model` explicitly opts in.
The first model-using call can download it if it is not cached. Network access,
disk space, and model-load time matter. `embed` is incremental; body updates
invalidate old vectors, so rerun it after changes and git synchronization.

Default CLI smart search can continue lexically when vectors or inference are
unavailable. **Forced** `--semantic` does not silently turn into keyword search:
a build without the feature errors, and runtime model errors propagate.

## Troubleshooting

**No results or stale body content.**
Check `forgeplan get ID` against the intended body. Use `update --body @file`
for a known artifact or `reindex` to recover external body edits. Check any
reported parse/duplicate-ID errors. `scan-import` is not a substitute.

**Russian inflections do not match.**
Try a longer query to help language detection, inspect the indexed body, and
compare with `--keyword` only if you want literal substring matching. Different
derivations need not share a stem; use semantic search for paraphrases.

**Semantic search misses a changed artifact.**
Run `forgeplan embed`; semantic-only output reports `unindexed_artifacts`.
A successful smart search may still have run without vector scores.

**Search is slow.**
Separate model startup/download from ranking cost. The historical v0.18
measurement is not a current performance threshold.

**A deleted file still appears.**
Run `forgeplan reindex` to trim orphan artifact rows and relations.

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
