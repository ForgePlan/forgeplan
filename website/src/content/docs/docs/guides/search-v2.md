---
title: Smart Search (v0.18 BM25 + Russian morphology)
description: "How Forgeplan's production BM25 engine finds artifacts — stemming, multi-language support, and CLI/MCP usage."
---

Forgeplan ships a production-grade search engine that finds artifacts by meaning,
not just by exact string match. v0.18.0 replaces the hand-written prototype with
the `bm25` crate (v2.3.2), adds Russian Snowball stemming, strips template noise
from the index, and cuts batch ranking from O(N²) to O(N). This guide explains
how it works and how to use it from the CLI and MCP.

## Who this is for

- You wrote an artifact months ago and only remember "something about auth".
- You work in a bilingual team (English + Russian) and want both queries to find
  the same PRD.
- You build an AI agent on top of Forgeplan's MCP server and need deterministic,
  fast retrieval.

## Production BM25 engine (v0.18.0)

Before v0.18 Forgeplan shipped a 140-line hand-written BM25 implementation. It
worked, but it was hard to tune, had no built-in stemmer, and scored records
one at a time — O(N²) in practice once the workspace grew past a hundred
artifacts. v0.18.0 replaces that prototype with the production-grade
[`bm25` crate v2.3.2](https://crates.io/crates/bm25), which brings four
concrete wins:

- **Proper term weighting.** Production TF-IDF / BM25 with length
  normalization, instead of our naive substring scoring.
- **O(N) batch search.** A single `search_scores()` call ranks the whole
  corpus in one pass. A 193-artifact workspace returns in **0.23s**,
  down from the multi-second stalls that made `search` feel laggy.
- **Built-in tokenizer stack.** Unicode segmentation, stop-word removal,
  and (crucially) pluggable stemmers — which is what enables Russian
  morphology in the next section.
- **Template noise stripping.** `strip_indexing_noise()` runs before
  tokenization and drops YAML frontmatter, `{placeholder}` lines,
  `|table|` rows, and HTML comments from the indexed text. This fixes
  PROB-030: on v0.17, `forgeplan search "auth"` would match every PRD
  that had `author:` in its frontmatter, flooding the results with
  false positives. v0.18 indexes only real body content.

These four changes land together in v0.18.0 and are covered by regression
tests in `crates/forgeplan-core/src/search/` — see the CHANGELOG entry.

## How it works

Smart search runs a three-stage pipeline on every query:

1. **Lexical (BM25)** — the `bm25` crate ranks documents by term frequency,
   inverse document frequency and length normalization. This is the default
   and covers the 90% case.
2. **Boosters** — exact-id match, kind filter, status filter, and recency
   nudge the lexical score before the final sort.
3. **Graph expansion** — results are optionally enriched with neighbors via
   typed links (`informs`, `refines`, `supersedes`) so a hit on a PRD also
   surfaces its RFC and evidence.

An optional semantic stage based on BGE-M3 embeddings can be layered on top —
see [Semantic search](#semantic-search-feature-flag) below.

### Tokenization and stemming

Every indexed document and every query runs through the same pipeline:

```
raw text
  → strip_indexing_noise()       // frontmatter, {placeholders}, |tables|, <!-- html -->
  → whichlang detect             // 17 languages, per-document and per-query
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
- No more "search feels laggy after 100 PRDs" reports

## Multi-language morphology

v0.18.0 enables `LanguageMode::Detect` in the BM25 tokenizer: each document
and each query is inspected by [`whichlang`](https://crates.io/crates/whichlang),
which picks the right Snowball stemmer on the fly. Seventeen languages are
supported out of the box — English, Russian, German, French, Spanish,
Italian, Portuguese, Dutch, Swedish, Norwegian, Danish, Finnish, Hungarian,
Romanian, Turkish, Arabic, and a generic fallback.

Because the stem is shared between every inflected form, all of the
following Russian queries return the same PRD:

```
$ forgeplan search "аутентификация"
PRD-019 Auth middleware    (matches "аутентификации" via stem "аутентификац")
EVID-023 Auth benchmark    (matches "аутентификацию")
```

The query `"аутентификация"` (nominative) and the stored form
`"аутентификации"` (genitive / dative) both normalize to the stem
`аутентификац`. Same mechanism, same scores, no configuration required in a
mixed-language workspace.

## Russian morphology in practice

Create a PRD in Russian:

```bash
forgeplan new prd "Система аутентификации"
```

Then search with any inflected form:

```bash
forgeplan search "аутентификация"     # nominative
forgeplan search "аутентификации"     # genitive / dative
forgeplan search "аутентификацией"    # instrumental
forgeplan search "аутентифицировать"  # verb form
```

All four queries return the same PRD with the same score, because the Snowball
Russian stemmer reduces every form to the shared stem. The same is true for
English (`authenticate`, `authentication`, `authenticating` all stem to `authent`).

Language is detected per document and per query via `whichlang`, so a mixed
workspace with English PRDs and Russian ADRs works without any configuration.

## CLI usage

All examples assume you are inside a Forgeplan workspace.

### 1. Simple query

```bash
forgeplan search "bm25 russian morphology"
```

Prints the top 10 hits with kind, id, title and score.

### 2. Filter by kind

```bash
forgeplan search "auth" --kind prd
forgeplan search "auth" --kind rfc
forgeplan search "auth" --kind evidence
```

Only artifacts of the requested kind are returned. Useful when you want the
RFC that explains how a PRD gets built.

### 3. Filter by status

```bash
forgeplan search "tags canonicalization" --status active
forgeplan search "deprecated semantics"   --status deprecated
```

Combine with `--kind` to scope precisely:

```bash
forgeplan search "scoring" --kind prd --status active
```

### 4. Limit results

```bash
forgeplan search "LanceDB" --limit 5
```

The default is 10. For scripting, set `--limit 1` to grab the single best hit.

### 5. Reindex after manual edits

If you edited markdown files outside the CLI (e.g. in your editor), rebuild
the BM25 index once:

```bash
forgeplan reindex
```

The reindex reads every file under `.forgeplan/`, re-runs
`strip_indexing_noise()`, and writes a fresh term frequency table. See PROB-027
for the canonical fix that removed the dependency on a live `lance/` folder
during reindex.

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

Response shape (truncated):

```json
{
  "hits": [
    {
      "id": "PRD-040",
      "kind": "prd",
      "title": "Scoring Intelligence",
      "status": "active",
      "score": 12.83,
      "path": ".forgeplan/prds/prd-040-scoring-intelligence.md"
    }
  ],
  "total": 1,
  "took_ms": 47
}
```

All CLI flags (`kind`, `status`, `limit`) are available as arguments. The
default if no filter is provided is "search all kinds, return top 10".

## Semantic search (feature flag)

For queries where lexical match is not enough (synonyms, paraphrasing,
cross-language similarity), Forgeplan can layer BGE-M3 dense embeddings on top
of BM25. This is an **opt-in** feature because embeddings add ~400 MB to the
binary and require a fastembed cache on first run.

Build from source with the feature:

```bash
cargo install forgeplan-cli --features semantic-search
```

Then search — the flag is transparent; BM25 still runs and semantic results
are merged in:

```bash
forgeplan search "how do I prove a decision is sound"
```

When the feature is off, the same command still works — Forgeplan falls back
to lexical-only search with a gentle log line. This graceful fallback is why
PRD-042 (FPF KB vector search) can ship the same binary to both minimal and
semantic users.

## Troubleshooting

**Score is always 0 or no results come back.**
Run `forgeplan reindex`. The most common cause is markdown edited outside the
CLI, so the index has stale term frequencies.

**Query in Russian finds nothing but the same English term works.**
Check that the document was indexed after v0.18.0 — earlier versions did not
have the Russian stemmer. Run `forgeplan reindex` once after upgrading.

**Too many irrelevant hits from frontmatter fields.**
You are on a pre-v0.18 binary. `strip_indexing_noise()` fixes this. Upgrade
and reindex.

**Search is slow on a large workspace.**
v0.18 is O(N) — a 193-artifact workspace should return in under a second. If
you see multi-second latency, file a PROB and attach the output of
`forgeplan health` plus a count of files under `.forgeplan/`.

**I deleted a file but it still shows up in results.**
Reindex. The BM25 store is derived state; deletions need to be replayed.

## Related artifacts

- **PRD-039** — Smart Search v2 (original v0.17 design)
- **PRD-040** — Scoring Intelligence (ranking signals feeding into search)
- **PRD-042** — FPF KB vector search (semantic search feature flag)
- **PROB-026** — Tag canonicalization (query-side normalization)
- **PROB-027** — Reindex without `lance/` folder
- **PROB-030** — `auth` prefix false positives from frontmatter
- **CHANGELOG v0.18.0** — production BM25 + Russian morphology release notes

## See also

- [`forgeplan search` CLI reference](/docs/cli/search/)
- [`forgeplan_search` MCP tool reference](/docs/mcp/forgeplan_search/)
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Ten rules of Forgeplan methodology](/docs/guides/ten-rules/)
