---
title: forgeplan progress
description: "Checkbox-based progress tracker for artifacts"
---

Parse checkbox lists (`- [ ]` / `- [x]`) inside artifact bodies and report
how much is done. The convention is that each functional requirement (FR)
or implementation phase in a PRD/RFC is a checkbox - so `progress` answers
"how much of this PRD is implemented?"

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

The text view groups artifacts by kind, with 24-cell Unicode bars, completed/total,
rounded integer percentage, and `WIP` or `DONE`. It also prints aggregate totals.
An ID selects a focused view with the individual checkbox lines.

`--json` contains `progress` and `_next_action`; each entry has `id`, `title`,
`kind`, `completed`, `total`, and `percent`. Both text and JSON omit artifacts
with zero checkboxes; a focused JSON query for one returns `progress: []`.

Recognized lines start with `- [ ] ` / `* [ ] ` or checked `x`/`X` equivalents
after leading whitespace. The parser counts the stored body line-by-line,
including checkbox-looking examples in code fences. It does not inspect code
or verify completed implementation. A 100% result may suggest `activate`, but
does not bypass activation guards.

## How it fits

`progress` tracks _execution_. Its sibling [`forgeplan score`](/docs/cli/score/)
tracks _quality_ (R_eff + F-G-R). A healthy PRD is both 100% and R_eff > 0.

```
progress  →  "are we done building?"
score     →  "do we trust it?"
health    →  aggregate both
```

## See also

- [`forgeplan score`](/docs/cli/score/) - R_eff quality metric
- [`forgeplan health`](/docs/cli/health/) - project rollup
- [`forgeplan list`](/docs/cli/list/) - find candidate IDs
