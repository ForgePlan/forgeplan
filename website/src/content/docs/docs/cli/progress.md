---
title: forgeplan progress
description: "Checkbox-based progress tracker for artifacts"
---

Parse checkbox lists (`- [ ]` / `- [x]`) inside artifact bodies and report
how much is done. The convention is that each functional requirement (FR)
or implementation phase in a PRD/RFC is a checkbox — so `progress` answers
"how much of this PRD is implemented?"

## When to use

- Sprint standups — quick percent complete per PRD
- Release prep — which PRDs are not 100% yet
- Reporting — pipe JSON into dashboards

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
forgeplan progress --json | jq '.[] | {id, percent}'
```

## Output interpretation

Default table:

```
ID        KIND  PERCENT  BAR                  DONE / TOTAL
PRD-001   prd     75%    [██████████████░░░░]   6 / 8
PRD-046   prd     40%    [████████░░░░░░░░░░]   4 / 10
RFC-004   rfc    100%    [██████████████████]   5 / 5
```

| Column     | Meaning                                              |
|------------|------------------------------------------------------|
| `PERCENT`  | `done / total` as integer percent                    |
| `BAR`      | 18-char ASCII progress bar                           |
| `DONE`     | Count of `- [x]` checkboxes                          |
| `TOTAL`    | Count of `- [x]` + `- [ ]` checkboxes                |

Artifacts with no checkboxes are omitted from the default view (there is
nothing to track). `--json` includes them with `total: 0`.

## How it fits

`progress` tracks _execution_. Its sibling [`forgeplan score`](/docs/cli/score/)
tracks _quality_ (R_eff + F-G-R). A healthy PRD is both 100% and R_eff > 0.

```
progress  →  "are we done building?"
score     →  "do we trust it?"
health    →  aggregate both
```

## See also

- [`forgeplan score`](/docs/cli/score/) — R_eff quality metric
- [`forgeplan health`](/docs/cli/health/) — project rollup
- [`forgeplan list`](/docs/cli/list/) — find candidate IDs
