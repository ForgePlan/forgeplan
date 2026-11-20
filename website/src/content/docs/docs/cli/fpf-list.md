---
title: forgeplan fpf list
description: "List ingested FPF sections by ID, line count and title."
---

`forgeplan fpf list` prints ingested **First Principles Framework** sections by ID, line count and title.

## When to use

- **For orientation** - scan the whole FPF structure in one screen.
- **Right after `fpf ingest`** - compare the list with the installed corpus revision.
- **To pick a section ID to read** before running [`fpf section`](/docs/cli/fpf-section/).
- **To sanity-check coverage** when a search returns surprisingly few results.

## When NOT to use

- For content lookup - use [`fpf search`](/docs/cli/fpf-search/).
- For reading a single section - use [`fpf section <id>`](/docs/cli/fpf-section/).
- For KB health / staleness - use [`fpf status`](/docs/cli/fpf-status/).

## Usage

```text
forgeplan fpf list [OPTIONS]
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

The corpus comes from an installed skill or an explicit `fpf ingest --path`.
Read that revision's headings rather than assuming a fixed three-part,
204-section edition. B.3 covers trust/assurance and B.5 covers the ADI reasoning
cycle in current upstream FPF; ForgePlan's F-G-R metric is a separate adaptation.

## How it fits

`fpf list` is the discovery entry point: scan the table, spot an interesting ID, then jump to [`fpf section`](/docs/cli/fpf-section/) or [`fpf search`](/docs/cli/fpf-search/) for deeper reading.

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/) - populate the KB first
- [`forgeplan fpf section`](/docs/cli/fpf-section/) - read one section
- [`forgeplan fpf search`](/docs/cli/fpf-search/) - find sections by content
- [`forgeplan fpf status`](/docs/cli/fpf-status/) - ingest health

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf list --help`.

```text
List all FPF sections

Usage: forgeplan fpf list

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
