---
title: forgeplan status
description: "Show workspace metadata, total artifacts, and separate counts by kind and lifecycle status."
---

`forgeplan status` prints the configured project name, workspace path, creation
timestamp, total artifact count, and separate counts by kind and status.
It does not print a kind×status matrix, recent activity, or sprint velocity.
Use `log` for recorded mutations, `journal` for a creation-date decision
snapshot, and `health` for integrity findings.

## When to use

- Getting oriented in an unfamiliar workspace
- Writing a weekly/sprint status report
- Confirming that `forgeplan reindex` picked up recently edited files
- Sanity check after a bulk import or merge
- Pre-standup — get current totals, then use `log` for recorded changes
- Demo - quick overview slide for stakeholders

## When NOT to use

- Detecting debt or quality issues - use `forgeplan health` (blind spots, orphans)
- Per-artifact inspection - use `forgeplan get <ID>` or `forgeplan context <ID>`
- CI gating - status has no exit-code semantics; use `health --ci`
- Finding a specific artifact - use `forgeplan search` or `forgeplan list`

## Usage

```text
forgeplan status
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: Basic snapshot

```bash
forgeplan status
```

Illustrative output for a workspace containing three records:

```text
Forgeplan Status
================

  Project:    Example API
  Workspace:  /path/to/project/.forgeplan
  Created:    2026-10-01T00:00:00Z
  Artifacts:  3 total

  By kind:
    adr              1
    prd              2

  By status:
    active           1
    draft            2
Next: forgeplan health
```

### Example 2: Post-import sanity check

```bash
forgeplan reindex
forgeplan status
```

Shows the number of indexed records after synchronizing workspace projections.
For external Markdown not yet imported, use `scan-import`; it is not a
replacement for reindexing projections already belonging to the workspace.

## Output interpretation

- **By kind** — counts grouped by stored kind.
- **By status** — independent counts grouped by stored lifecycle status.
  Normal lifecycle states are `draft`, `active`, `stale`, `superseded`,
  and `deprecated`; this renderer prints stored values rather than validating them.
- There is no recent-activity section and no JSON option in CLI 0.38.0.

Red flags:

- Many drafts and no active - you create artifacts but never finish them
- Counts alone cannot establish whether a project is abandoned or how fast it is moving.
- Evidence count disproportionately low vs PRD count - missing evidence
  coverage; run `forgeplan health` for specifics
- Inspect expiry dates with `stale`; do not assume every Note has automatic expiry or requires no review.

## How it fits the workflow

```
session start → [status] → health → route → Shape → ...
                   ^
             optional warmup
```

- **Status** answers "what exists?"
- **Health** answers "what is broken?"
- Run status for orientation, then health for the actionable list
- Together they form the session-start dashboard pair

## See also

- [`forgeplan health`](/docs/cli/health/) - problem detection and next actions
- [`forgeplan list`](/docs/cli/list/) - filterable artifact listing
- [`forgeplan search`](/docs/cli/search/) - keyword / semantic search
- [`forgeplan context`](/docs/cli/context/) - per-artifact full view
- [`forgeplan journal`](/docs/cli/journal/) - full decision history
