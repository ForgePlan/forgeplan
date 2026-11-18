---
title: forgeplan progress
description: "Checkbox-based progress tracker for artifacts"
---

Counts checked and unchecked task markers in artifact bodies. This reports author-marked completion, not proof that a requirement was implemented or accepted.

## When to use

- Sprint standups - quick percent complete per PRD
- Release prep - which PRDs are not 100% yet
- Reporting - pipe JSON into dashboards

## Not to use when

- You need artifact _quality_ (not completion) → use [`forgeplan score`](/docs/cli/score/)
- You need _health_ rollup → use [`forgeplan health`](/docs/cli/health/)

## Usage

```text
forgeplan progress [OPTIONS] [ID]
```

## Arguments

```text
  [ID]  Artifact ID (shows all if omitted)
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

All artifacts, default table:

```bash
forgeplan progress
```

Just one PRD:

```bash
forgeplan progress PRD-001
```

Machine-readable for a dashboard:

```bash
forgeplan progress --json | jq '.progress[] | {id, percent}'
```

## Output interpretation

Illustrative table, not a captured runtime layout:

```
ID        KIND  PERCENT  BAR                  DONE / TOTAL
PRD-001   prd     75%    [██████████████░░░░]   6 / 8
PRD-046   prd     40%    [████████░░░░░░░░░░]   4 / 10
RFC-004   rfc    100%    [██████████████████]   5 / 5
```

| Column     | Meaning                                              |
|------------|------------------------------------------------------|
| `PERCENT`  | `done / total` as integer percent                    |
| `BAR`      | 24-position filled/empty block progress bar           |
| `DONE`     | Count of `- [x]` checkboxes                          |
| `TOTAL`    | Count of `- [x]` + `- [ ]` checkboxes                |

Artifacts with no checkboxes are omitted from both the table and the JSON `progress` array.

## How it fits

`progress` counts marked tasks. [`score`](/docs/cli/score/) evaluates declared evidence and [`health`](/docs/cli/health/) reports debt; neither proves implementation completion. A 100% checkbox count and positive R_eff alone are not an acceptance criterion.

## See also

- [`forgeplan score`](/docs/cli/score/) - R_eff quality metric
- [`forgeplan health`](/docs/cli/health/) - project rollup
- [`forgeplan list`](/docs/cli/list/) - find candidate IDs

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan progress --help`.

```text
Show checkbox progress for artifacts

Usage: forgeplan progress [OPTIONS] [ID]

Arguments:
  [ID]  Artifact ID (shows all if omitted)

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
