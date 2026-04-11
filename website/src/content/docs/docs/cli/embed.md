---
title: forgeplan embed
description: "Generate vector embeddings for all artifacts (semantic search)"
---

Generate vector embeddings for every artifact in the workspace so semantic
search can find related work by meaning, not just keywords. Embeddings are
stored alongside the LanceDB metadata and used by `forgeplan search --semantic`
and semantic MCP tools.

## Usage

```text
forgeplan embed
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Feature flag

`embed` is gated behind the `semantic-search` Cargo feature flag. The default
Forgeplan binary ships **without** semantic search to keep the download small
and avoid bundling an ML model. To enable it, build from source:

```bash
cargo install --path crates/forgeplan-cli --features semantic-search
```

Without the flag the command is a no-op stub that prints a hint pointing to
BM25 keyword search (which is already production-grade as of v0.18.0).

## What it does

1. Loads the BGE-M3 embedding model via [fastembed-rs](https://github.com/Anush008/fastembed-rs).
2. Walks every artifact in LanceDB and concatenates title + body + tags.
3. Computes a 1024-dim vector per artifact.
4. Writes vectors to the `embedding` column of the `artifacts` table.
5. Caches the model weights in `.forgeplan/.fastembed_cache/` (gitignored).

First run downloads ~500MB of model weights — subsequent runs reuse the cache.
Embedding a typical 100-artifact workspace takes 10-60 seconds depending on
your hardware (CPU fallback by default, CUDA/Metal if available).

## When you need it

- You want **semantic search** — "find artifacts about trust decay" should
  surface R_eff / evidence work even if those exact words don't appear.
- You're building **AI agent workflows** where an MCP tool retrieves related
  context by similarity rather than keyword.
- You use **FPF KB vector search** (PRD-042) against the methodology corpus.

## When you don't need it

- Your corpus is small (<50 artifacts) — keyword search (BM25 + Russian
  morphology, v0.18.0) finds everything.
- You prefer a minimal install without an ML dependency.
- You want reproducible builds without a ~500MB model download on first run.

Default `forgeplan search "query"` now uses production BM25 (v0.18.0) which
handles Russian stemming, English, and template noise stripping — it's a
strong keyword baseline and may be enough on its own.

## Example

```bash
# Build with semantic search enabled
cargo install --path crates/forgeplan-cli --features semantic-search

# Generate embeddings for the whole workspace
forgeplan embed

# Now semantic search works
forgeplan search "trust calculus" --semantic
```

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan search`](/docs/cli/search/) — keyword + semantic retrieval
- [`forgeplan reindex`](/docs/cli/reindex/) — rebuild metadata (doesn't touch embeddings)
- [`forgeplan fpf search`](/docs/cli/fpf-search/) — search FPF knowledge base
