---
title: forgeplan fpf status
description: "Show FPF knowledge base status - source, ingested section count, staleness"
---

`forgeplan fpf status` reports the discovered local FPF source directory, its Markdown file count, and ingested section and line counts. It does not record a last-ingest timestamp or compare spec versions or modification times.

## When to use

- **Right after `fpf ingest`** - verify the expected section count loaded.
- **When `fpf search` returns zero results** - confirm the KB is actually populated.
- **After a Forgeplan upgrade** - check if a re-ingest is needed.
- **On fresh clones** - because `.forgeplan/lance/` is gitignored, new checkouts start with an empty KB.
- **In CI smoke tests** - assert the KB is ready before running reasoning flows.

## When NOT to use

- For artifact-level project health - use [`forgeplan health`](/docs/cli/health/).
- For a full section listing - use [`fpf list`](/docs/cli/fpf-list/).

## Usage

```text
forgeplan fpf status
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

- **Source** — currently discovered default source path, or instructions for supplying `fpf ingest --path <dir>`. This is not a stored record of the path used for the last ingest.
- **Files** — `.md` entries directly inside the source directory's immediate child directories, excluding `_index.md`, when the count is nonzero. Root-level Markdown and deeper nested entries are not counted; this is not a recursive scan.
- **Ingested** — not initialized, empty, or section count plus total line count.
- **Status** — `STALE` when a nonzero source-file count differs from ingested section count, otherwise `UP TO DATE` when the source count is nonzero. This count heuristic cannot detect changed content with an unchanged count; source layouts outside the one-directory-level scan can also make it misleading.

There is no semantic-search readiness or embedding-completeness field. Missing source or an empty KB is reported as text and does not itself cause a failing exit status.

## How it fits

Use this inspection before `fpf search` or reasoning that needs KB excerpts. It is not a CI gate by exit code. `fpf check` uses configured or built-in rules independently of KB ingest.

```
fpf ingest  →  fpf status (verify)  →  fpf search / reason --fpf
```

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/) - populate or refresh the KB
- [`forgeplan fpf list`](/docs/cli/fpf-list/) - content-level browse
- [`forgeplan health`](/docs/cli/health/) - project-level counterpart
