---
title: forgeplan fpf section
description: "Show a specific FPF section by ID (e.g. B.3 for Trust Calculus)"
---

`forgeplan fpf section <ID>` prints the full body of one **First Principles Framework** section, addressed by its canonical ID (like `B.3` or `A.1.2`). It's the `less` of the FPF KB - use it when a search result is interesting and you want to read the whole thing.

## When to use

- **After a promising `fpf search` hit** - read the full section, not just the snippet.
- **When a methodology doc or `forgeplan reason` output cites an FPF ID** - jump straight to the source.
- **While writing ADRs or RFCs** - quote the section you're relying on verbatim.

## When NOT to use

- For discovery - use [`forgeplan fpf search`](/docs/cli/fpf-search/) when you don't yet know the section ID.
- For the full index - use [`forgeplan fpf list`](/docs/cli/fpf-list/).

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
# Read Trust Calculus in the ingested revision; distinguish it from ForgePlan's R_eff adaptation
forgeplan fpf section B.3

# Just the first 500 chars when you only need the gist
forgeplan fpf section B.3 --summary

# Read another section by ID; inspect its title in your ingested revision
forgeplan fpf section B.4

# Abduction–Deduction–Induction (ADI), FPF B.5
forgeplan fpf section B.5
```

## How it fits

Sections are retrieved by `fpf search`, loaded by `fpf ingest`, and cited by FPF rules. FPF B.5 describes ADI. `forgeplan reason --fpf` injects relevant FPF sections into the ADI prompt, but on a Constellation-managed workspace the legacy FPF corpus is retired, so that path reports no FPF sections instead of reopening the retired store. ForgePlan's R_eff is a product adaptation, not arithmetic mandated by the current upstream B.3.

A typical workflow:

```bash
forgeplan fpf search "congruence level"   # find candidates
forgeplan fpf section B.3                 # read the winner in full
forgeplan new adr "Evidence grading policy"
# ...cite B.3 in the ADR body
```

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) - parent command
- [`forgeplan fpf search`](/docs/cli/fpf-search/) - find sections by content
- [`forgeplan fpf list`](/docs/cli/fpf-list/) - all sections at a glance
- [Methodology guide](/docs/methodology/overview/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf section --help`.

```text
Show a specific FPF section

Usage: forgeplan fpf section [OPTIONS] <ID>

Arguments:
  <ID>  Section ID (e.g. "B.3", "C.2.2")

Options:
      --summary  Show summary only (first 500 chars)
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
