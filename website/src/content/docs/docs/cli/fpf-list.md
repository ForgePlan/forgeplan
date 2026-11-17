---
title: forgeplan fpf list
description: "List ingested FPF sections in a table with Section, Lines, and Title columns."
---

`forgeplan fpf list` prints the sections currently ingested into the workspace's **First Principles Framework** knowledge base. Columns are `Section`, `Lines`, and `Title`, followed by the section count. There is no part column or JSON flag.

## When to use

- **For orientation** - scan the whole FPF structure in one screen.
- **Right after `fpf ingest`** - compare the loaded count with the corpus you selected.
- **To pick a section ID to read** before running [`fpf section`](/docs/cli/fpf-section/).
- **To sanity-check coverage** when a search returns surprisingly few results.

## When NOT to use

- For content lookup - use [`fpf search`](/docs/cli/fpf-search/).
- For reading a single section - use [`fpf section <id>`](/docs/cli/fpf-section/).
- For KB health / staleness - use [`fpf status`](/docs/cli/fpf-status/).

## Usage

```text
forgeplan fpf list
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
# Full table
forgeplan fpf list

# Typical browse flow
forgeplan fpf list
forgeplan fpf section B.3
forgeplan fpf search "trust"
```

## Structure of the FPF corpus

The corpus comes from an installed FPF skill or an explicit `fpf ingest --path`
directory; it is not a fixed 204-section payload bundled in the CLI.
Section IDs and titles are derived from source headings or filenames.
Use the actual IDs returned by `list` rather than assuming every installation
has the same parts or section count.

An empty KB is a successful result with a message instructing you to run
`forgeplan fpf ingest` first.
The populated table's next-action hint points at its first section.

## How it fits

`fpf list` is the discovery entry point: scan the table, spot an interesting ID, then jump to [`fpf section`](/docs/cli/fpf-section/) or [`fpf search`](/docs/cli/fpf-search/) for deeper reading.

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/) - populate the KB first
- [`forgeplan fpf section`](/docs/cli/fpf-section/) - read one section
- [`forgeplan fpf search`](/docs/cli/fpf-search/) - find sections by content
- [`forgeplan fpf status`](/docs/cli/fpf-status/) - ingest health
