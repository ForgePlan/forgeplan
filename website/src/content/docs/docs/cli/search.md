---
title: forgeplan search
description: "Smart search - BM25 + semantic + graph expansion with Russian morphology"
---

Smart search across artifacts. As of v0.18.0, Forgeplan ships **production
BM25** (via the `bm25` crate v2.3.2) with Russian Snowball stemming,
template-noise stripping, and 1-hop graph expansion. Semantic vectors are
used as a ranker booster when the `semantic-search` feature flag is enabled.

## When to use

- You remember a concept but not the ID
- Onboarding - "what do we have on authentication?"
- Drafting a new artifact - find related prior decisions to link
- AI agents doing retrieval-augmented reasoning

## Not to use when

- You know the exact ID → use [`forgeplan get`](/docs/cli/get/)
- You want a structural view → use [`forgeplan tree`](/docs/cli/tree/) or
  [`forgeplan graph`](/docs/cli/graph/)
- You want to search personal memories, not artifacts → use
  [`forgeplan recall`](/docs/cli/recall/)

## Usage

```text
forgeplan search [OPTIONS] <QUERY>
```

## Arguments

```text
  <QUERY>  Search query
```

## Options

```text
  -t, --type <TYPE>      Filter by kind (prd, rfc, adr, note, ...)
  -s, --status <STATUS>  Filter by status (draft, active, superseded, deprecated, stale)
      --depth <DEPTH>    Filter by depth (tactical, standard, deep, critical)
      --with-evidence    Only artifacts with evidence linked (R_eff > 0)
      --no-evidence      Only artifacts WITHOUT evidence (blind spots)
      --since <SINCE>    Only artifacts created after this date (YYYY-MM-DD)
      --no-expand        Disable graph expansion (1-hop neighbors in results)
      --keyword          Force keyword-only search (substring grep)
      --semantic         Force semantic-only search (vector similarity)
  -n, --limit <LIMIT>    Max results to return (default: 20) [default: 20]
      --json             Output as JSON for machine consumption
  -h, --help             Print help
  -V, --version          Print version
```

## Examples

Smart default — BM25/substring keyword channels, optional semantic scores, and graph expansion:

```bash
forgeplan search "authentication flow"
```

Narrow to active PRDs only, top 5 hits:

```bash
forgeplan search "auth" --type prd --status active --limit 5
```

Find decisions that have no evidence backing them (a blind-spot variant):

```bash
forgeplan search "rate limit" --no-evidence
```

## Output interpretation

Smart text output starts with `Found N result(s) … (smart search)`.
Each hit shows the combined score, `ID [kind|status] "title"`, then
`kw`, `sem`, `r`, and `g` signal values. Expanded hits add
`via <id> (expanded neighbor)`. Scores rank one query's results; they are
not comparable across queries. Stemming groups word forms, not arbitrary
synonyms or every derivative.

`--json` returns `results` and `_next_action`. Smart hits contain `id`,
`kind`, `status`, `title`, `score`, `keyword_score`, `semantic_score`, `r_eff`,
`graph_centrality`, `expanded_from` (nullable), `mode: "smart"`, and
`semantic_available`. Text `kw` is the maximum of BM25 and substring scores;
JSON `keyword_score` is only the substring score.

### Mode-specific limits in 0.38.0

- Default smart mode applies the composite filters and `--limit`.
  Evidence filters use cached `r_eff_score > 0` / `== 0`, not a fresh
  scan of linked packs. `--since` is strictly after midnight UTC on the date.
- `--keyword` uses substring matching and only the kind filter. It ignores
  status/depth/evidence/date filtering and `--limit` (#544), although an
  invalid `--since` date is still rejected before mode dispatch. Its JSON
  hits have `id`, `kind`, `status`, `title`, and `mode: "keyword"`, not scores.
- `--semantic` requires semantic-search support and embeddings. It retrieves
  up to 50 vector candidates, applies kind, and returns at most 10. It also
  ignores composite filters and `--limit`. JSON hits add `similarity`,
  `distance`, and `mode: "semantic"`; the envelope adds `unindexed_artifacts`.
  Artifacts without vectors cannot appear.
- `--no-expand` disables both neighbor expansion and the centrality booster
  in smart mode. Smart mode degrades to keyword/graph ranking when semantic
  initialization or retrieval fails; first use may download the embedding model.

Empty results succeed with `results: []`. Empty queries and malformed
`--since` dates are errors. MCP search is a separate surface; do not assume
its filters or score fields match these CLI mode contracts.

## How it fits

`search` is the primary discovery surface:

```
search → get → link (during artifact authoring)
search → get → reason (during AI planning)
```

For the full pipeline details (indexing, stemming, BM25 parameters, graph
expansion), see the [Search v2 guide](/docs/guides/search-v2/).

## See also

- [Search v2 guide](/docs/guides/search-v2/) - architecture and tuning
- [`forgeplan get`](/docs/cli/get/) - read a hit in full
- [`forgeplan recall`](/docs/cli/recall/) - search memories (not artifacts)
- [`forgeplan reindex`](/docs/cli/reindex/) - rebuild the BM25 index
