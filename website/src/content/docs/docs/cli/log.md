---
title: forgeplan log
description: "Show change log — audit trail of artifact mutations"
---

Show the change log — an append-only audit trail of every artifact mutation
in the workspace. Each entry records who/what touched which artifact, when,
and via which source (CLI command, file edit, git sync, or reindex).

## When to use

- "What changed in the last day?" — morning standup context
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

Only CLI-originated mutations, last 50 — filter out noise from file edits:

```bash
forgeplan log --source cli --limit 50
```

## Output interpretation

Each line is one event:

```
2026-04-11 14:32  cli        PRD-001   activate       draft → active
2026-04-11 14:28  cli        PRD-001   link           → EVID-012 (informs)
2026-04-11 14:20  file_edit  PRD-001   body_update    (detected by scan-import)
2026-04-11 13:55  cli        EVID-012  create         EvidencePack created
```

| Column       | Meaning                                                |
|--------------|--------------------------------------------------------|
| Timestamp    | Local time of the mutation                             |
| Source       | `cli`, `file_edit`, `git_sync`, `reindex`             |
| Artifact ID  | What changed                                           |
| Action       | create, update, link, activate, supersede, deprecate, etc. |
| Detail       | Human-readable delta                                   |

If git-sync is enabled, entries may include a short commit hash in the detail
column, tying the mutation to a git commit.

## How it fits

`log` is the low-level mutation trail. The higher-level views filter it:

```
log        (everything that happened)
  ↓
journal    (only decisions with R_eff)
  ↓
health     (aggregated project state)
```

## See also

- [`forgeplan journal`](/docs/cli/journal/) — decision-only timeline
- [`forgeplan health`](/docs/cli/health/) — aggregated dashboard
- [`forgeplan session`](/docs/cli/session/) — current workflow phase
