---
title: forgeplan fpf
description: "First Principles Framework Knowledge Base — ingest, search, and apply FPF rules to artifacts"
---

`forgeplan fpf` is the parent command for the **First Principles Framework (FPF) Knowledge Base** — a 204-section reasoning corpus that powers Forgeplan's ADI cycle, trust calculus, and explore/investigate/exploit rule engine.

FPF is the theoretical backbone behind `forgeplan reason`, R_eff scoring, and methodology integrity checks. The `fpf` subcommands let you load the spec, query it semantically, inspect active rules, and check how those rules apply to specific artifacts.

## When to use

- **Once per workspace** — run `fpf ingest` after `forgeplan init` so the KB is available locally.
- **During reasoning** — `fpf search "trust calculus"` to pull first-principles context while shaping a PRD or ADR.
- **During sprint planning** — `fpf dashboard` to see bounded contexts, quality scores, and explore-vs-exploit recommendations.
- **During validation** — `fpf check PRD-XXX` to see which FPF rules fire on an artifact and what action they suggest.
- **For onboarding** — `fpf list` + `fpf section B.3` to read the spec directly from the CLI.

## When NOT to use

- For general artifact CRUD — use `forgeplan new`, `validate`, `review`, `activate` instead.
- For project-wide health — use `forgeplan health`, not `fpf dashboard` (the two are complementary, not interchangeable).
- For ADI reasoning runs — use `forgeplan reason --fpf`, which internally consults the KB; direct `fpf search` is for humans inspecting raw content.

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
  dashboard  Show FPF dashboard — bounded contexts, quality scores, explore-exploit actions
  ingest     Ingest FPF spec into knowledge base
  search     Search FPF knowledge base
  section    Show a specific FPF section
  list       List all FPF sections
  status     Show FPF knowledge base status — source, ingested count, staleness
  rules      List active FPF rules grouped by action (EXPLORE/INVESTIGATE/EXPLOIT)
  check      Check which FPF rules match a given artifact
  help       Print this message or the help of the given subcommand(s)
```

## Examples

```bash
# One-time setup after forgeplan init
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

- **PRD-041** wires FPF rules into the route/validate stages.
- **PRD-042** adds BGE-M3 vector search across the 204 sections (the same pipeline as the artifact search — BM25 + semantic fusion).
- **PRD-043** enforces methodology integrity: an artifact that violates bounded-context or trust-calculus rules is flagged before activation.

The KB is stored in LanceDB under `.forgeplan/lance/` (derived, gitignored). Raw sections live in the Forgeplan repo and are embedded on ingest.

## See also

- [`forgeplan fpf dashboard`](/docs/cli/fpf-dashboard/) — bounded contexts + explore/exploit overview
- [`forgeplan fpf ingest`](/docs/cli/fpf-ingest/) — one-time KB load
- [`forgeplan fpf search`](/docs/cli/fpf-search/) — semantic search over FPF sections
- [`forgeplan fpf section`](/docs/cli/fpf-section/) — read a specific section
- [`forgeplan fpf list`](/docs/cli/fpf-list/) — all sections
- [`forgeplan fpf status`](/docs/cli/fpf-status/) — KB health and staleness
- [`forgeplan fpf rules`](/docs/cli/fpf-rules/) — active rules by action
- [`forgeplan fpf check`](/docs/cli/fpf-check/) — rules matching an artifact
- [`forgeplan reason`](/docs/cli/reason/) — ADI cycle driven by FPF
- [Methodology guide](/docs/methodology/overview/) — where FPF fits in the full workflow
