---
title: forgeplan fpf ingest
description: "Ingest the FPF spec into the local knowledge base (one-time setup)"
---

`forgeplan fpf ingest` loads the **First Principles Framework** specification into the workspace knowledge base. It parses the 204-section corpus (A/B/C parts), chunks it, generates embeddings via the same BGE-M3 pipeline used by artifact search, and writes the result into a LanceDB table under `.forgeplan/lance/`.

Run it **once per workspace**, typically right after `forgeplan init -y`. After ingest, `fpf search`, `fpf section`, `fpf list`, `fpf rules`, and `fpf check` all become available.

## When to use

- **Right after `forgeplan init -y`** — seed the KB before doing any reasoning work.
- **After wiping `.forgeplan/lance/`** — because lance is gitignored, fresh clones need re-ingest.
- **After a Forgeplan upgrade** that ships new FPF sections — re-ingest to refresh the corpus.
- **When `fpf status` reports the KB as stale or empty.**

## When NOT to use

- You don't need to run it before every reasoning session; the KB persists across invocations.
- Don't run it in parallel with `forgeplan reason --fpf` — let ingest finish first.
- Not a replacement for `forgeplan scan-import` — that rebuilds the **artifact** index, not the FPF KB.

## Usage

```text
forgeplan fpf ingest [OPTIONS]
```

## Options

```text
      --path <PATH>  Path to FPF sections directory
  -h, --help         Print help
  -V, --version      Print version
```

By default, `fpf ingest` reads the FPF spec bundled inside the Forgeplan binary. Pass `--path` to ingest from an external sections directory instead — useful for local FPF spec development or testing a patched corpus.

## Examples

```bash
# First-time setup — uses bundled spec
forgeplan init -y
forgeplan fpf ingest

# Re-ingest after upgrade
forgeplan fpf ingest
forgeplan fpf status

# Ingest from an external FPF sections directory
forgeplan fpf ingest --path ./fpf-sections/
```

## What happens

1. Forgeplan locates the bundled FPF spec (ships inside the binary).
2. Sections are parsed into structured chunks (ID, title, part, body).
3. Embeddings are generated via BGE-M3 (feature-gated — falls back gracefully if `semantic-search` is disabled).
4. LanceDB writes one row per section into the `fpf_kb` table.
5. A status record is stamped with ingest timestamp and section count.

On a warm fastembed cache the whole operation takes a few seconds; cold runs pay the model download cost once.

## How it fits

Ingest is the bootstrap step for everything else under `forgeplan fpf`:

```
forgeplan fpf ingest      ← one-time
  ├── fpf search          ← works after ingest
  ├── fpf section B.3
  ├── fpf list
  ├── fpf rules
  ├── fpf check PRD-XXX
  └── fpf dashboard
```

It's also a prerequisite for `forgeplan reason --fpf`, which pulls first-principles context into the ADI prompt.

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) — parent command
- [`forgeplan fpf status`](/docs/cli/fpf-status/) — verify ingest succeeded
- [`forgeplan fpf search`](/docs/cli/fpf-search/) — query the ingested KB
- [`forgeplan init`](/docs/cli/init/) — workspace bootstrap that precedes ingest
