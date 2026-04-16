---
title: forgeplan coverage
description: "Show decision coverage per code module — which parts of the codebase have no linked ADR/PRD."
---

`forgeplan coverage` cross-references the modules discovered by `forgeplan scan`
against the `Affected Files` sections in every artifact, and reports which modules
have **no** documented decision. Uncovered modules are blind spots: code you are
running without any documented reasoning behind it.

This is the "do we know why this exists?" view of the project. Paired with `drift`
(do ADRs still match code?) it gives you a complete codebase ⟷ artifacts reconciliation.

## When to use

- Architecture review: "which modules are entirely undocumented?"
- After a refactor that split or merged modules — rescan, recompute coverage.
- Onboarding: show a new teammate which files have ADRs they should read first.
- Backfilling legacy: use `--backfill` to auto-insert `Affected Files` sections on artifacts that pre-date the module link requirement.

## When NOT to use

- Before running `forgeplan scan` — coverage needs a module inventory first.
- On tiny projects with 1–2 files — the signal is low.

## Usage

```text
forgeplan coverage [OPTIONS]
```

## Options

```text
      --backfill  Backfill "Affected Files" section into artifacts missing it
  -h, --help      Print help
  -V, --version   Print version
```

## Examples

### Standard coverage report

```bash
forgeplan scan         # refresh module list
forgeplan coverage
```

Output:

```text
Decision coverage
─────────────────
covered (5):
  crates/forgeplan-core/src/scoring      ADR-001, PRD-005
  crates/forgeplan-core/src/lifecycle    ADR-005
  crates/forgeplan-core/src/db           ADR-002, ADR-003
  crates/forgeplan-core/src/search       PRD-039, RFC-006
  crates/forgeplan-cli/src/commands      RFC-001

uncovered (3) ⚠ blind spots:
  crates/forgeplan-core/src/fpf
  crates/forgeplan-core/src/routing
  crates/forgeplan-mcp/src/transport

overall coverage: 62% (5 of 8 modules)
```

### Backfill legacy artifacts

```bash
forgeplan coverage --backfill
```

Adds a placeholder `## Affected Files` section to artifacts that don't have one,
making them visible to coverage and drift going forward. You'll still need to fill
the list by hand, but the structure is ready.

### Focus on uncovered modules

```bash
forgeplan coverage --json | jq '.uncovered[]'
```

Produces a flat list you can feed into a backlog generator: "write an ADR for each
uncovered module."

## Output interpretation

| Section   | Meaning                                                            |
|-----------|--------------------------------------------------------------------|
| covered   | module → list of artifacts declaring it in `Affected Files`        |
| uncovered | module with zero artifact references — document reasoning!        |
| overall   | percentage of scanned modules with at least one decision           |

A healthy project sits above 70% coverage. Below 50% means you're coding by instinct.
100% is usually over-documentation — some utility modules don't need ADRs.

## How it fits the workflow

```
scan → coverage → spot uncovered module → forgeplan new adr → link affected files → re-run
```

Coverage is the "what should I document next?" tool. It turns architecture review
from vibes into a prioritized list.

## See also

- [`forgeplan scan`](/docs/cli/scan/) — populate the module inventory
- [`forgeplan drift`](/docs/cli/drift/) — the other half of codebase reconciliation
- [`forgeplan new adr`](/docs/cli/new/) — create the missing documentation
- [`forgeplan health`](/docs/cli/health/) — includes coverage in project-level summary
- [CLI overview](/docs/cli/)
