---
title: forgeplan log
description: "Show change log - audit trail of artifact mutations"
---

Shows recorded artifact-change entries. Entries carry timestamp, artifact, action, field changes, source, and optional commit hash. This is not a guaranteed complete audit of every mutation.

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

Illustrative historical events, not a captured transcript or the exact current column layout:

```
2026-04-11 14:32  cli        PRD-001   activate       draft → active
2026-04-11 14:28  cli        PRD-001   link           → EVID-012 (informs)
2026-04-11 14:20  file_edit  PRD-001   body_update    (recorded file synchronization)
2026-04-11 13:55  cli        EVID-012  create         EvidencePack created
```

The current text columns are **TIMESTAMP**, **ARTIFACT**, **ACTION**, **FIELD**, **CHANGE**, **SOURCE**, and **COMMIT**. The timestamp is a shortened stored timestamp, not a guaranteed conversion to local time. `CHANGE` is truncated old/new field data; a Git hash has its own column.

## How it fits

`log` reads historical change entries. `journal` builds a decision timeline and `health` computes present workspace signals from records and relations; neither is a filter over the log. For MCP invocation records, use [`forgeplan activity`](/docs/cli/activity/), which is a separate journal.

## See also

- [`forgeplan journal`](/docs/cli/journal/) - decision-only timeline
- [`forgeplan health`](/docs/cli/health/) - aggregated dashboard
- [`forgeplan session`](/docs/cli/session/) - current workflow phase

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan log --help`.

```text
Show change log — audit trail of artifact mutations

Usage: forgeplan log [OPTIONS] [ID]

Arguments:
  [ID]  Filter by artifact ID

Options:
  -n, --limit <LIMIT>    Maximum number of entries (default: 20) [default: 20]
      --source <SOURCE>  Filter by source (cli, file_edit, git_sync, reindex)
      --json             Output as JSON for machine consumption
  -h, --help             Print help
  -V, --version          Print version
```
<!-- forgeplan-runtime:end -->
