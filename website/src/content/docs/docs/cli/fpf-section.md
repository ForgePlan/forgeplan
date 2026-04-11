---
title: forgeplan fpf section
description: "Show a specific FPF section by ID (e.g. B.3 for Trust Calculus)"
---

`forgeplan fpf section <ID>` prints the full body of one **First Principles Framework** section, addressed by its canonical ID (like `B.3` or `A.1.2`). It's the `less` of the FPF KB — use it when a search result is interesting and you want to read the whole thing.

## When to use

- **After a promising `fpf search` hit** — read the full section, not just the snippet.
- **When a methodology doc or `forgeplan reason` output cites an FPF ID** — jump straight to the source.
- **While writing ADRs or RFCs** — quote the section you're relying on verbatim.

## When NOT to use

- For discovery — use [`forgeplan fpf search`](/docs/cli/fpf-search/) when you don't yet know the section ID.
- For the full index — use [`forgeplan fpf list`](/docs/cli/fpf-list/).

## Usage

```text
forgeplan fpf section [OPTIONS] <ID>
```

## Arguments

```text
  <ID>   Section ID (e.g. "B.3", "C.2.2")
```

## Options

```text
      --summary  Show summary only (first 500 chars)
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

```bash
# Trust calculus (the FPF B.3 section that defines R_eff semantics)
forgeplan fpf section B.3

# Just the first 500 chars when you only need the gist
forgeplan fpf section B.3 --summary

# Explore/exploit reasoning
forgeplan fpf section B.4

# ADI cycle
forgeplan fpf section C.1
```

## How it fits

Sections are the atomic unit of the FPF KB. They're what `fpf search` ranks, what `fpf ingest` chunks, what `fpf check` cites when explaining why a rule matched, and what `forgeplan reason --fpf` pulls in as context.

A typical workflow:

```bash
forgeplan fpf search "congruence level"   # find candidates
forgeplan fpf section B.3                 # read the winner in full
forgeplan new adr "Evidence grading policy"
# ...cite B.3 in the ADR body
```

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) — parent command
- [`forgeplan fpf search`](/docs/cli/fpf-search/) — find sections by content
- [`forgeplan fpf list`](/docs/cli/fpf-list/) — all sections at a glance
- [Methodology guide](/docs/methodology/overview/)
