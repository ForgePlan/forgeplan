---
title: forgeplan status
description: "Workspace dashboard with total artifacts and separate kind/status counts"
---

`forgeplan status` prints project/workspace metadata, the artifact total and
separate counts by kind and by status. It does not show a kind×status cross-tab
or recent activity. For problems use `health`; for historical events inspect
the applicable journal or change-log command.

Use it for orientation and current counts, not to measure sprint velocity.

## When to use

- Getting oriented in an unfamiliar workspace
- Writing a weekly/sprint status report
- Confirming artifact counts after importing foreign documents
- Sanity check after a bulk import or merge
- Pre-standup — inspect the current inventory
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

The text dashboard has these sections:

- Project, Workspace, Created and Artifacts metadata
- By kind — counts per artifact kind
- By status — counts per stored lifecycle state

It does not print the dated recent-activity rows or synthetic `resolved`/`expired`
states shown in older examples. Counts and dates must come from the actual workspace.

### Example 2: Post-import sanity check

```bash
forgeplan scan-import
forgeplan status
```

Compare the expected artifact count after an import. `scan-import` has no
`--verbose`; preview foreign documents with `--dry-run`. For canonical-file
maintenance use the owning index pipeline, not another external import.

### Example 3: Sprint standup one-liner

```bash
forgeplan status | head -15
```

Truncates the text dashboard; the result is not a stable machine-readable schema.
No recent-activity tail is produced by this command.

## Output interpretation

- **By kind** and **By status** are independent totals, not a cross-tab.
- Core stored statuses are `draft`, `active`, `stale`, `superseded`, `deprecated`.
- Counts alone do not prove a project is abandoned, healthy or moving quickly.
- A low evidence count can motivate inspection; use `health` and the actual links
  rather than deriving evidence coverage from a simple ratio.
- This dashboard does not implement or verify automatic Note expiration.

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
- [`forgeplan context`](/docs/cli/context/) - per-artifact metadata, validation, scoring and links, without the body
- [`forgeplan journal`](/docs/cli/journal/) - decision journal

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan status --help`.

```text
Show project status dashboard

Usage: forgeplan status

Options:
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
