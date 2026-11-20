---
title: forgeplan fpf status
description: "Show FPF knowledge base status - source, ingested section count, staleness"
---

`forgeplan fpf status` compares the Markdown file count at the default installed FPF source path with the ingested section count. It reports counts, not revision or embedding readiness.

Think of it as `forgeplan health` for the FPF KB specifically.

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

Reported fields:

- **Source / Files** - default skill path and Markdown count, when found.
- **Ingested** - section count and total lines, or empty/not initialized.
- **Status** - `STALE` if the nonzero source count differs; otherwise `UP TO DATE`.

Matching counts do not detect edits within files. The command does not remember
an arbitrary previous `--path`, display last-ingest time, or prove semantic availability.

## How it fits

Use this as inspection, not a semantic readiness gate.

```
fpf ingest → fpf status (counts) → inspect/search the intended corpus
```

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/) - populate or refresh the KB
- [`forgeplan fpf list`](/docs/cli/fpf-list/) - content-level browse
- [`forgeplan health`](/docs/cli/health/) - project-level counterpart

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf status --help`.

```text
Show FPF knowledge base status — source, ingested count, staleness

Usage: forgeplan fpf status

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
