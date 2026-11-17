---
title: forgeplan reopen
description: "Deprecate an artifact AND create a new draft successor - lineage-preserving re-evaluation of a past decision."
---

`forgeplan reopen` creates a new draft of the same kind, links it to the source with `based_on`, then deprecates the old artifact with your reason. Use it for re-evaluation rather than a validity extension. It is a sequence of writes, not a promise of atomic rollback if a later write fails. Unlike `supersede`, it creates the replacement instead of referring to one that already exists.

## Forgeplan 0.38.0

The source can be active or stale; terminal statuses are refused. The new draft has a based_on relation to the old artifact, which becomes deprecated. This creates a successor, not a transition from terminal back to draft.

## When to use

- An ADR went stale and a review concluded the approach itself needs rethinking from the ground up.
- A PRD's original assumptions no longer match reality and you want to rewrite it rather than patch it.
- A RefreshReport flagged multiple red signals on a previously active decision - reopen to start a fresh draft.
- A ProblemCard you thought was solved has resurfaced in a new form and you need a new artifact to re-scope it.

## When NOT to use

- The decision is still valid and just needs a fresh expiry - use [`forgeplan renew`](/docs/cli/renew/).
- You already have an active replacement - use [`forgeplan supersede`](/docs/cli/supersede/) with `--by`.
- The decision is simply retired with nothing to take its place - use [`forgeplan deprecate`](/docs/cli/deprecate/).
- You need a small amendment - edit the artifact directly or create a follow-up artifact without disturbing the current one.

## Usage

```text
forgeplan reopen --reason <REASON> <ID>
```

## Arguments

```text
  <ID>  Artifact ID to reopen
```

## Options

```text
      --reason <REASON>  Reason for reopening
  -h, --help             Print help
  -V, --version          Print version
```

## Examples

### Example 1: Reopen an ADR for full re-evaluation

```bash
forgeplan reopen ADR-007 --reason "storage strategy needs rethinking after LanceDB v0.8 changes"
```

`ADR-007` enters `deprecated` (terminal), and a fresh ADR draft is created with a lineage link back to `ADR-007`. Fill it out and take it through the normal Shape → Validate → ADI cycle.

### Example 2: Reopen a PRD whose assumptions drifted

```bash
forgeplan reopen PRD-010 --reason "target users changed after 2026 Q1 persona research"
```

The draft inherits kind, depth, author, parent epic, and tags, resets validity, and uses the old title with ` (reopened)` appended. Its body contains a Lineage section, not the old content or a full template. The old PRD remains readable with a Reopened section.

### Example 3: Use reopen as part of a refresh cycle

```bash
forgeplan stale
forgeplan review ADR-012
forgeplan reopen ADR-012 --reason "review identified 3 invalidated invariants"
```

Combine with a `forgeplan new refresh` artifact when the re-evaluation needs a structured report before the new draft is opened.

## How it fits the workflow

Reopen is the most expensive lifecycle transition: it restarts the `Shape → Validate → ADI → Code → Evidence → Activate` cycle for an entire decision. After reopening, you should treat the new draft as a full-depth artifact - fill in MUST sections, run `forgeplan reason` if the depth warrants it, produce fresh evidence, and only then activate. The old artifact survives in terminal state so links from historical context (other artifacts, git history, prior discussions) still resolve. Reach for it when a simple `renew` would hide a real change, and when there isn't yet a successor to `supersede` with.

## Common errors

| Error | Cause | Fix |
|---|---|---|
| `--reason is required` | Flag omitted | Pass `--reason "..."` explaining why re-evaluation is needed |
| `Cannot reopen …: status 'draft'. Reopen requires active or stale.` | Source is draft | Edit that draft directly |
| `Cannot reopen …: status '…' is terminal.` | Source is deprecated or superseded | Create a new artifact with `forgeplan new` |
| New draft contains only lineage | Expected behavior, not an error | Write required content, then validate |

## See also

- [`forgeplan renew`](/docs/cli/renew/) - extend validity when the decision is still correct
- [`forgeplan supersede`](/docs/cli/supersede/) - replace with an already-active successor
- [`forgeplan deprecate`](/docs/cli/deprecate/) - retire with no follow-up
- [`forgeplan new`](/docs/cli/new/) - manually scaffold a fresh artifact if reopen doesn't fit
- [`forgeplan activate`](/docs/cli/activate/) - activate the new draft once it's ready
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Methodology: Artifact Lifecycle](/docs/methodology/lifecycle/)
