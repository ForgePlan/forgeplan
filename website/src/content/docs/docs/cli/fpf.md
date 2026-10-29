---
title: forgeplan fpf
description: "First Principles Framework Knowledge Base - ingest, search, and apply FPF rules to artifacts"
---

`forgeplan fpf` groups **First Principles Framework (FPF) Knowledge Base** ingestion, search, section inspection, dashboards, and executable rule inspection. The section count follows the installed source you ingest; it is not a fixed 204-section guarantee.

FPF informs the product's reasoning conventions, but ForgePlan's numerical scoring and rules are local adaptations, not a literal implementation of every upstream FPF rule. The KB and executable rules are separate surfaces.

## When to use

- **After workspace initialization** - install the FPF section source or pass `fpf ingest --path` to make the KB available.
- **During reasoning** - `fpf search "trust calculus"` to pull first-principles context while shaping a PRD or ADR.
- **During sprint planning** - `fpf dashboard` to see bounded contexts, quality scores, and explore-vs-exploit recommendations.
- **During validation** - `fpf check PRD-XXX` to see which FPF rules fire on an artifact and what action they suggest.
- **For onboarding** - `fpf list` + `fpf section B.3` to read the spec directly from the CLI.

## When NOT to use

- For general artifact CRUD - use `forgeplan new`, `validate`, `review`, `activate` instead.
- For project-wide health - use `forgeplan health`, not `fpf dashboard` (the two are complementary, not interchangeable).
- For ADI runs use `forgeplan reason --fpf`; enrichment is conditional. The selected migrated-workspace path omits legacy KB context.

## Usage

```text
forgeplan fpf <COMMAND>
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Subcommands

```text
  dashboard  Show FPF dashboard - bounded contexts, quality scores, explore-exploit actions
  ingest     Ingest FPF spec into knowledge base
  search     Search FPF knowledge base
  section    Show a specific FPF section
  list       List all FPF sections
  status     Show FPF knowledge base status - source, ingested count, staleness
  rules      List active FPF rules grouped by action (EXPLORE/INVESTIGATE/EXPLOIT)
  check      Check which FPF rules match a given artifact
  help       Print this message or the help of the given subcommand(s)
```

## Examples

```bash
# Ingest installed FPF sections; otherwise supply --path to their directory
forgeplan fpf ingest

# Explore the KB
forgeplan fpf status
forgeplan fpf list
forgeplan fpf section B.3

# Pull first-principles context into your reasoning
forgeplan fpf search "trust calculus"
forgeplan fpf search "bounded context"

# Apply FPF rules to the project and specific artifacts
forgeplan fpf dashboard
forgeplan fpf rules
forgeplan fpf check PRD-019
```

## How it fits

FPF sits at the **reasoning layer** of the Forgeplan pipeline:

```
Shape → Validate → ADI (FPF KB) → Code → Evidence → Activate
```

- **PRD-041** introduced the rule-inspection surface.
- **PRD-042** added optional BGE-M3 vector lookup for ingested sections; it is separate from artifact BM25 retrieval.
- **PRD-043** concerns methodology integrity; listing KB sections is not proof that lifecycle gates passed.

Ingest reads installed FPF sections or an explicit `--path` and stores a derived `fpf_spec` table under `.forgeplan/lance/`. A semantic-search build attempts embeddings during ingestion; keyword-only builds store text. `fpf search`/`section`/`list` can read that auxiliary table without recreating artifact tables.

## See also

- [`forgeplan fpf dashboard`](/docs/cli/fpf-dashboard/) - bounded contexts + explore/exploit overview
- [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/) - one-time KB load
- [`forgeplan fpf search`](/docs/cli/fpf-search/) - semantic search over FPF sections
- [`forgeplan fpf section`](/docs/cli/fpf-section/) - read a specific section
- [`forgeplan fpf list`](/docs/cli/fpf-list/) - all sections
- [`forgeplan fpf status`](/docs/cli/fpf-status/) - KB health and staleness
- [`forgeplan fpf rules`](/docs/cli/fpf-rules/) - active rules by action
- [`forgeplan fpf check`](/docs/cli/fpf-check/) - rules matching an artifact
- [`forgeplan reason`](/docs/cli/reason/) - ADI cycle driven by FPF
- [Methodology guide](/docs/methodology/overview/) - where FPF fits in the full workflow

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan fpf --help`.

```text
FPF Knowledge Base — dashboard, ingest, search, sections

Usage: forgeplan fpf <COMMAND>

Commands:
  dashboard  Show FPF dashboard — bounded contexts, quality scores, explore-exploit actions
  ingest     Ingest FPF spec into knowledge base
  search     Search FPF knowledge base
  section    Show a specific FPF section
  list       List all FPF sections
  status     Show FPF knowledge base status — source, ingested count, staleness
  rules      List active FPF rules grouped by action (EXPLORE/INVESTIGATE/EXPLOIT)
  check      Check which FPF rules match a given artifact
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

- [`forgeplan fpf dashboard`](/docs/cli/fpf-dashboard/)
- [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/)
- [`forgeplan fpf search`](/docs/cli/fpf-search/)
- [`forgeplan fpf section`](/docs/cli/fpf-section/)
- [`forgeplan fpf list`](/docs/cli/fpf-list/)
- [`forgeplan fpf status`](/docs/cli/fpf-status/)
- [`forgeplan fpf rules`](/docs/cli/fpf-rules/)
- [`forgeplan fpf check`](/docs/cli/fpf-check/)
<!-- forgeplan-runtime:end -->
