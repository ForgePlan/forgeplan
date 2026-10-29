---
title: forgeplan coverage
description: "Show decision coverage per code module - which parts of the codebase have no linked ADR/PRD."
---

`forgeplan coverage` scans the codebase itself, then matches module paths against
`Affected Files` in active PRD, RFC and ADR artifacts. An uncovered module has no
matching decision in that set; it is a documentation-review candidate, not proof
that its code has no rationale.

This is the "do we know why this exists?" view of the project. Paired with `drift`
(do ADRs still match code?) it helps review codebase ⟷ artifact relationships.

## When to use

- Architecture review: "which modules are entirely undocumented?"
- After a refactor that split or merged modules - rescan, recompute coverage.
- Onboarding: show a new teammate which files have ADRs they should read first.
- Backfilling legacy: use `--backfill` to auto-insert `Affected Files` sections on artifacts that pre-date the module link requirement.

## When NOT to use

- You need a persisted module inventory — coverage scans on each invocation; `scan` is an optional preview.
- On tiny projects with 1–2 files - the signal is low.

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
forgeplan scan         # optional preview; coverage performs its own scan
forgeplan coverage
```

Illustrative report (invented modules/counts, not literal CLI output):

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

Adds a placeholder `## Affected Files` section to active PRD/RFC/ADR artifacts
that lack one. Fill it with actual paths; the placeholder is not evidence of
real coverage.

### Focus on uncovered modules

```bash
forgeplan coverage
```

Inspect the uncovered-module section in the text report. This command has no
`--json` option in the selected build.

## Output interpretation

| Section   | Meaning                                                            |
|-----------|--------------------------------------------------------------------|
| covered   | module → list of artifacts declaring it in `Affected Files`        |
| uncovered | module with zero artifact references - document reasoning!        |
| overall   | percentage of scanned modules with at least one decision           |

Choose documentation priorities from module risk and importance. The CLI does
not define universal healthy/unhealthy thresholds of 70%, 50% or 100% coverage.

## How it fits the workflow

```
scan → coverage → spot uncovered module → forgeplan new adr → link affected files → re-run
```

Coverage is the "what should I document next?" tool. It turns architecture review
from vibes into a prioritized list.

## See also

- [`forgeplan scan`](/docs/cli/scan/) - preview source modules without persisting an inventory
- [`forgeplan drift`](/docs/cli/drift/) - the other half of codebase reconciliation
- [`forgeplan new adr`](/docs/cli/new/) - create the missing documentation
- [`forgeplan health`](/docs/cli/health/) - separate artifact-health summary, not a coverage threshold
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan coverage --help`.

```text
Show decision coverage per code module

Usage: forgeplan coverage [OPTIONS]

Options:
      --backfill  Backfill "Affected Files" section into artifacts missing it
  -h, --help      Print help
  -V, --version   Print version
```
<!-- forgeplan-runtime:end -->
