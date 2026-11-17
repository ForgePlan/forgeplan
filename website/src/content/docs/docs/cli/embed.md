---
title: forgeplan embed
description: "Generate semantic-search vectors for artifacts whose title and body have changed. Content hashes let the command skip up-to-date embeddings. If there is nothing to embed, it returns without loading the model."
---

Generate semantic-search vectors for artifacts whose title and body have changed. Content hashes let the command skip up-to-date embeddings. If there is nothing to embed, it returns without loading the model.

## Usage

```text
forgeplan embed
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Published binaries and cache

Forgeplan 0.38.0 release binaries include semantic search. BGE-M3 weights are a separate first-use download of about 2.1 GB. The released tract engine uses CPU inference, not CUDA or Metal. A custom build without `semantic-search` rejects this command; it is not a successful no-op.

## What it does

The command embeds the title plus the first `embedding.chunk_size` body characters (default 2000), then updates one vector and a content hash per artifact in LanceDB. It truncates the body preview; it does not create multiple searchable chunks. BGE-M3 produces 1024-dimensional vectors. This command does not populate the separate FPF knowledge base.

## Cache location

The model is shared across projects: `~/Library/Caches/forgeplan/models` on macOS, `~/.cache/forgeplan/models` on Linux, and the platform cache directory on Windows. `FORGEPLAN_MODEL_CACHE` overrides it. Project-local `.fastembed_cache/` is legacy, not the default.

## Example

```bash
# Optional: download the model before the first search.
forgeplan setup --skip-alias --skip-hooks
forgeplan embed
forgeplan search "trust calculus" --semantic
```

## When keyword search is enough

BM25 keyword search does not require embeddings. Use it when downloads, network access, or inference cost are undesirable. Missing or unusable model files can prevent semantic search; do not treat an embedding failure as success.

Per-artifact encoding failures are printed as `FAIL` and counted in the final summary, but the loop still returns success. Inspect the failed count and stderr rather than using exit status alone as proof that every artifact was embedded.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan search`](/docs/cli/search/) - keyword + semantic retrieval
- [`forgeplan reindex`](/docs/cli/reindex/) - scan projections and sync bodies/new records; not a full metadata refresh for existing records or an embedding refresh
- [`forgeplan fpf search`](/docs/cli/fpf-search/) - search FPF knowledge base
