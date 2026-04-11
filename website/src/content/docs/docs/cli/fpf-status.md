---
title: forgeplan fpf status
description: "Show FPF knowledge base status — source, ingested section count, staleness"
---

`forgeplan fpf status` reports the health of the local **First Principles Framework** knowledge base: where it was loaded from, how many sections are in LanceDB, when it was last ingested, and whether it's stale relative to the shipped spec.

Think of it as `forgeplan health` for the FPF KB specifically.

## When to use

- **Right after `fpf ingest`** — verify the expected section count loaded.
- **When `fpf search` returns zero results** — confirm the KB is actually populated.
- **After a Forgeplan upgrade** — check if a re-ingest is needed.
- **On fresh clones** — because `.forgeplan/lance/` is gitignored, new checkouts start with an empty KB.
- **In CI smoke tests** — assert the KB is ready before running reasoning flows.

## When NOT to use

- For artifact-level project health — use [`forgeplan health`](/docs/cli/health/).
- For a full section listing — use [`fpf list`](/docs/cli/fpf-list/).

## Usage

```text
forgeplan fpf status [OPTIONS]
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
# Check KB status
forgeplan fpf status

# Common "empty workspace" flow
forgeplan init -y
forgeplan fpf status       # → not ingested
forgeplan fpf ingest
forgeplan fpf status       # → ingested, N sections
```

## What you see

Typical fields reported:

- **Source** — path / version of the FPF spec that was ingested.
- **Ingested sections** — number of rows in the `fpf_kb` LanceDB table.
- **Last ingest** — timestamp (or "never").
- **Staleness** — whether the shipped spec is newer than the ingested data.
- **Semantic search** — whether the `semantic-search` feature is enabled and embeddings are present.

## How it fits

`fpf status` is a **gate command**: it answers the yes/no question "is my FPF KB usable right now?" before you run `fpf search`, `fpf check`, or `forgeplan reason --fpf`.

```
fpf ingest  →  fpf status (verify)  →  fpf search / reason --fpf
```

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) — parent command
- [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/) — populate or refresh the KB
- [`forgeplan fpf list`](/docs/cli/fpf-list/) — content-level browse
- [`forgeplan health`](/docs/cli/health/) — project-level counterpart
