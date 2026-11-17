---
title: forgeplan log
description: "Show change log - audit trail of artifact mutations"
---

Show recorded artifact changes from the workspace change-log table, newest
first. Entries identify the artifact, action, field delta, source, timestamp,
and optional commit; they do not contain a dedicated actor/author field.
This is not a guarantee that every file edit or operation was logged.

## When to use

- "What changed in the last day?" - morning standup context
- Debugging why a score or status shifted unexpectedly
- Reviewing a teammate's activity on a shared workspace
- Verifying a git-sync or reindex pass worked

## Not to use when

- You want _decisions only_ (not all mutations) → use [`forgeplan journal`](/docs/cli/journal/)
- You want project health rollup → use [`forgeplan health`](/docs/cli/health/)
- You want to search _content_ → use [`forgeplan search`](/docs/cli/search/)

## Usage

```text
forgeplan log [OPTIONS] [ID]
```

## Arguments

```text
  [ID]  Filter by artifact ID
```

## Options

```text
  -n, --limit <LIMIT>    Maximum number of entries (default: 20) [default: 20]
      --source <SOURCE>  Filter by source (cli, file_edit, git_sync, reindex)
      --json             Output as JSON for machine consumption
  -h, --help             Print help
  -V, --version          Print version
```

## Examples

Last 20 events across all artifacts (default):

```bash
forgeplan log
```

History of one specific PRD:

```bash
forgeplan log PRD-001
```

Only CLI-originated mutations, last 50 - filter out noise from file edits:

```bash
forgeplan log --source cli --limit 50
```

## Output interpretation

Text columns are `TIMESTAMP`, `ARTIFACT`, `ACTION`, `FIELD`, `CHANGE`,
`SOURCE`, and `COMMIT`. Timestamps are shortened to the stored date/minute,
not converted to local time. `CHANGE` is a truncated old/new field delta;
body changes may use hashes. Commit hashes have their own column.

`--json` returns `entries` and `_next_action`. Each entry has `timestamp`,
`artifact_id`, `action`, `field`, `old_value`, `new_value`, `source`, and
`commit_hash`; optional values are null rather than omitted. An empty query
is successful and returns `entries: []`. Filtering is by the supplied ID
and source; there is no time-window flag on this command.

## How it fits

`log` reads recorded mutations. `journal` independently summarizes current
artifact records by creation date, and `health` independently analyzes current
workspace state; neither is a filter over this change log.

## See also

- [`forgeplan journal`](/docs/cli/journal/) - decision-only timeline
- [`forgeplan health`](/docs/cli/health/) - aggregated dashboard
- [`forgeplan session`](/docs/cli/session/) - current workflow phase
