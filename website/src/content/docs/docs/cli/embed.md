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

The command requires the `semantic-search` Cargo feature. Source installation
must opt in explicitly:

```bash
cargo install --path crates/forgeplan-cli --features semantic-search
```

A build without the feature refuses with an installation hint. Generating vectors requires the compatible model snapshot; semantic retrieval then also requires compatible vectors published in the workspace index. Existing vectors are not a prerequisite for creating them.

## What it does

The local `0.39.0+constellation.s1.1` build uses the pure-Rust `tract` engine,
not the earlier fastembed runtime. Only `bge-m3` is supported (1024 dimensions).
On a migrated workspace, `embed` drains embedding jobs for the published
Constellation generation, including prefix and section units; it is not
limited to one vector per artifact.

If no generation is published, this command bootstraps index maintenance first; that can migrate a legacy workspace. Prepare the complete cache before invoking it. The interface below describes the selected `0.39.0+constellation.s1.1` binary, not the unchanged installed `0.38.0` release.

The model is roughly 2.1 GB and is cached outside the project. The cache
priority is `FORGEPLAN_MODEL_CACHE` → `~/.forgeplan/models` → platform cache
or legacy relative fallback only when no home is available. `HF_HOME` does
not override the tract cache path. Pass the cache root, not a snapshot path.

Queries and background maintenance do not download missing weights.
`forgeplan setup` is the explicit preparation step. Never delete the model
cache as a diagnostic shortcut.

## Automatic embeddings

A manual `embed` does not persist `embedding:` configuration. For automatic
queue draining, configure `model: bge-m3` explicitly, prepare the complete
snapshot, and run maintenance. `serve --auto-index` alone is not enough.
See [Constellation setup](/docs/guides/constellation/).

## When you need it

- You want **semantic search** - "find artifacts about trust decay" should
  surface R_eff / evidence work even if those exact words don't appear.
- You're building **AI agent workflows** where an MCP tool retrieves related
  context by similarity rather than keyword.
- You need artifact embeddings; FPF KB ingestion is a separate operation.

## When you don't need it

- Keyword search is sufficient for your task.
- You prefer a build without the semantic-search feature.
- You cannot provision the model download or its disk space.

Default `forgeplan search` is smart search, with keyword and available semantic signals. Use `--keyword` when you explicitly need keyword-only retrieval; it does not require model weights. Do not infer semantic readiness from a successful keyword result.

## Example

```bash
# Build with semantic search enabled
cargo install --path crates/forgeplan-cli --features semantic-search

# Explicitly prepare the complete model snapshot
forgeplan setup --skip-alias --skip-hooks

# Generate embeddings for the whole workspace
forgeplan embed

# Query after embeddings have been published
forgeplan search "trust calculus" --semantic
```

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan search`](/docs/cli/search/) - keyword + semantic retrieval
- [Constellation reference](/docs/reference/constellation/) - embedding coverage and identity
- [`forgeplan fpf search`](/docs/cli/fpf-search/) - search FPF knowledge base

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan embed --help`.

```text
Generate embeddings for all artifacts (semantic search)

Usage: forgeplan embed

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
