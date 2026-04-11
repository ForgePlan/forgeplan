---
title: forgeplan fpf list
description: "List all ingested FPF sections as a table (ID, part, title)"
---

`forgeplan fpf list` prints every section in the **First Principles Framework** knowledge base as a table — section ID, part (A/B/C), and title. It's the table-of-contents view of the KB.

## When to use

- **For orientation** — scan the whole FPF structure in one screen.
- **Right after `fpf ingest`** — confirm the expected ~204 sections are present.
- **To pick a section ID to read** before running [`fpf section`](/docs/cli/fpf-section/).
- **To sanity-check coverage** when a search returns surprisingly few results.

## When NOT to use

- For content lookup — use [`fpf search`](/docs/cli/fpf-search/).
- For reading a single section — use [`fpf section <id>`](/docs/cli/fpf-section/).
- For KB health / staleness — use [`fpf status`](/docs/cli/fpf-status/).

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

Sections are grouped into three parts:

- **Part A — Foundations.** Kernel architecture, reasoning primitives, base vocabulary.
- **Part B — Trust calculus.** Evidence grading, congruence levels, R_eff semantics, explore/investigate/exploit.
- **Part C — ADI cycle.** Abduction → Deduction → Induction, hypothesis management, adversarial review.

A total of 204 sections are shipped with Forgeplan and loaded into the KB on `fpf ingest`.

## How it fits

`fpf list` is the discovery entry point: scan the table, spot an interesting ID, then jump to [`fpf section`](/docs/cli/fpf-section/) or [`fpf search`](/docs/cli/fpf-search/) for deeper reading.

## See also

- [`forgeplan fpf`](/docs/cli/fpf/) — parent command
- [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/) — populate the KB first
- [`forgeplan fpf section`](/docs/cli/fpf-section/) — read one section
- [`forgeplan fpf search`](/docs/cli/fpf-search/) — find sections by content
- [`forgeplan fpf status`](/docs/cli/fpf-status/) — ingest health
