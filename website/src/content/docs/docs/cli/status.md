---
title: forgeplan status
description: "Quick summary dashboard — kind × status breakdown and recent activity"
---

`forgeplan status` prints a compact snapshot of the workspace: how many
artifacts of each kind exist, what their lifecycle status is, and what has
changed recently. It is the "what's in this project?" command, not the
"what's broken?" command — for problem detection use `forgeplan health`.

Use it when you open a project you have not touched in a week, when you
want to see sprint velocity at a glance, or when you are writing a status
report and need the current counts.

## When to use

- Getting oriented in an unfamiliar workspace
- Writing a weekly/sprint status report
- Confirming that `forgeplan scan-import` picked up recently edited files
- Sanity check after a bulk import or merge
- Pre-standup — see what moved since yesterday
- Demo — quick overview slide for stakeholders

## When NOT to use

- Detecting debt or quality issues — use `forgeplan health` (blind spots, orphans)
- Per-artifact inspection — use `forgeplan get <ID>` or `forgeplan context <ID>`
- CI gating — status has no exit-code semantics; use `health --ci`
- Finding a specific artifact — use `forgeplan search` or `forgeplan list`

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

Typical output:

```
Forgeplan Workspace: /Users/me/Work/forgeplan
================================================

Artifacts by kind:
  PRD       42  (draft: 3, active: 35, superseded: 4)
  RFC       18  (draft: 2, active: 14, deprecated: 2)
  ADR       12  (active: 10, superseded: 2)
  Epic       4  (active: 4)
  Spec       8  (active: 6, draft: 2)
  Evidence  56  (active: 56)
  Problem    5  (active: 3, resolved: 2)
  Note      20  (active: 14, expired: 6)
  --------
  Total    165

Recent activity (last 7 days):
  2026-04-10  PRD-042  created   "OAuth2 login flow"
  2026-04-10  RFC-018  activated
  2026-04-09  EVID-056 linked to PRD-041
  2026-04-08  ADR-012  activated
```

### Example 2: Post-import sanity check

```bash
forgeplan scan-import
forgeplan status
```

Confirms that the expected number of artifacts was picked up after manual
markdown edits. If counts look wrong, re-run `scan-import` with `--verbose`.

### Example 3: Sprint standup one-liner

```bash
forgeplan status | head -15
```

Dump the header and counts into a chat message without the recent-activity
tail.

## Output interpretation

- **Artifacts by kind** — totals per ArtifactKind, split by lifecycle status.
  `draft`, `active`, `stale`, `superseded`, `deprecated`, `expired` are the
  possible states. Draft-heavy means unfinished work; superseded-heavy means
  healthy evolution.
- **Recent activity** — last 7 days of create / update / link / activate /
  supersede events. Pulled from the decision journal.

Red flags:

- Many drafts and no active — you create artifacts but never finish them
- Zero recent activity over multiple days — project may be abandoned or work
  happening outside the CLI (direct markdown edits without `scan-import`)
- Evidence count disproportionately low vs PRD count — missing evidence
  coverage; run `forgeplan health` for specifics
- Notes with many `expired` entries — auto-expiration working as intended,
  no action needed

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

- [`forgeplan health`](/docs/cli/health/) — problem detection and next actions
- [`forgeplan list`](/docs/cli/list/) — filterable artifact listing
- [`forgeplan search`](/docs/cli/search/) — keyword / semantic search
- [`forgeplan context`](/docs/cli/context/) — per-artifact full view
- [`forgeplan journal`](/docs/cli/journal/) — full decision history
