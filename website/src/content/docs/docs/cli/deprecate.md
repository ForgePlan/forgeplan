---
title: forgeplan deprecate
description: "Retire an active or stale artifact without a replacement — terminal transition recording why it no longer applies."
---

`forgeplan deprecate` moves an artifact from `active` (or `stale`) to `deprecated` — a terminal state. Unlike supersede, deprecation does not point at a replacement: you are saying "this decision no longer applies and nothing is taking its place." The `--reason` flag is required so future readers understand why the artifact was retired, and the artifact itself is preserved so historical links keep resolving.

## When to use

- A business pivot made a PRD irrelevant: the feature was cancelled and there is no follow-up PRD.
- A ProblemCard was closed because the underlying issue disappeared (environment changed, workaround became permanent) and no SolutionPortfolio is needed.
- A constraint ADR no longer applies: the technology it governed has been removed from the stack entirely.
- A `stale` artifact was reviewed and the team decided it should not be renewed — the validity genuinely expired.

## When NOT to use

- You have a direct replacement — use [`forgeplan supersede`](/docs/cli/supersede/) with `--by` so the lineage is tracked.
- You want to re-evaluate the decision in a fresh draft — use [`forgeplan reopen`](/docs/cli/reopen/), which combines deprecation with creation of a successor draft.
- The artifact is still in `draft` — drafts that were never activated can be removed or rewritten, not deprecated.
- You only need to extend validity on a stale artifact — use [`forgeplan renew`](/docs/cli/renew/).

## Usage

```text
forgeplan deprecate --reason <REASON> <ID>
```

## Arguments

```text
  <ID>  Artifact ID
```

## Options

```text
      --reason <REASON>  Reason for deprecation
  -h, --help             Print help
  -V, --version          Print version
```

## Examples

### Example 1: Retire a cancelled PRD

```bash
forgeplan deprecate PRD-011 --reason "feature cancelled after Q1 roadmap review, no successor"
```

`PRD-011` enters `deprecated`. Health reports exclude it from blind-spot calculations but it remains readable.

### Example 2: Close an obsolete ProblemCard

```bash
forgeplan deprecate PROB-018 --reason "root cause removed by infra migration, problem no longer reproducible"
```

ProblemCards don't run the MUST gate on activation, but deprecation still records the terminal reason.

### Example 3: Deprecate a stale ADR that will not be renewed

```bash
forgeplan stale
forgeplan deprecate ADR-009 --reason "valid_until expired 2026-03; architecture replaced by RFC-014 scope"
```

Use `forgeplan stale` to find expired artifacts, then deprecate the ones that shouldn't be renewed.

## How it fits the workflow

Deprecation is a clean, terminal ending: the artifact leaves the active set and stops influencing `forgeplan health`. It is the right tool when a decision's relevance has genuinely ended and no successor exists. Because the state is terminal, the reason you pass becomes the permanent explanation — write it for a reader six months from now, not for yourself today.

## Common errors

| Error | Cause | Fix |
|---|---|---|
| `--reason is required` | Flag omitted | Pass `--reason "..."` with a human-readable explanation |
| `Cannot deprecate from draft` | Artifact was never activated | Delete or rewrite the draft instead |
| `Already in terminal state` | Already `deprecated` or `superseded` | Terminal states are final — nothing to do |
| `Blind spots increased after deprecation` | Active artifacts depended on this one | Update or supersede dependents before deprecating |

## See also

- [`forgeplan supersede`](/docs/cli/supersede/) — retire with a replacement link
- [`forgeplan reopen`](/docs/cli/reopen/) — deprecate and auto-create a new draft
- [`forgeplan renew`](/docs/cli/renew/) — extend validity on a stale artifact instead of deprecating
- [`forgeplan stale`](/docs/cli/stale/) — find artifacts whose `valid_until` expired
- [`forgeplan health`](/docs/cli/health/) — verify the project state after deprecation
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Methodology: Artifact Lifecycle](/docs/methodology/lifecycle/)
