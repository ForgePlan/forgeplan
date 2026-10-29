---
title: forgeplan reopen
description: "Deprecate an artifact AND create a new draft successor - lineage-preserving re-evaluation of a past decision."
---

`forgeplan reopen` starts re-evaluation of an active or stale decision. It creates a same-kind draft, links the new artifact `based_on` the original, then deprecates the original with your reason. These are ordered writes, not an atomic simultaneous transition: a failure can leave a new draft before retirement completes. Fill the draft and follow the applicable Shape → Validate → ADI → Code → Evidence → Activate workflow.

## When to use

- An ADR went stale and a review concluded the approach itself needs rethinking, not just a validity extension.
- A PRD's original assumptions no longer match reality and you want to rewrite it rather than patch it.
- A RefreshReport flagged multiple red signals on a previously active decision - reopen to start a fresh draft.
- A ProblemCard you thought was solved has resurfaced in a new form and you need a new artifact to re-scope it.

## When NOT to use

- The decision is still valid and just needs a fresh expiry - use [`forgeplan renew`](/docs/cli/renew/).
- You already have an active replacement - use [`forgeplan supersede`](/docs/cli/supersede/) with `--by`.
- The decision is simply retired with nothing to take its place - use [`forgeplan deprecate`](/docs/cli/deprecate/).
- You need a small amendment — use `forgeplan update` or create a follow-up without retiring the current artifact.

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

The new draft inherits the kind (`PRD`) and is linked as the successor under re-evaluation. The old PRD stays readable for historical context.

### Example 3: Use reopen as part of a refresh cycle

```bash
forgeplan stale
forgeplan new refresh "Review ADR-012"
forgeplan reopen ADR-012 --reason "refresh report identified 3 invalidated invariants"
```

There is no `forgeplan refresh` command. Create and fill a RefreshReport with `new refresh` when a structured review is needed before reopening.

## How it fits the workflow

Reopen is the most expensive lifecycle transition: it restarts the `Shape → Validate → ADI → Code → Evidence → Activate` cycle for an entire decision. After reopening, you should treat the new draft as a full-depth artifact - fill in MUST sections, run `forgeplan reason` if the depth warrants it, produce fresh evidence, and only then activate. The old artifact survives in terminal state so links from historical context (other artifacts, git history, prior discussions) still resolve. Reach for it when a simple `renew` would hide a real change, and when there isn't yet a successor to `supersede` with.

## Common errors

| Error | Cause | Fix |
|---|---|---|
| `--reason is required` | Flag omitted | Pass `--reason "..."` explaining why re-evaluation is needed |
| `Cannot reopen from draft` | Original never activated | Edit the existing draft directly instead |
| `Cannot reopen from terminal state` | Already `deprecated` or `superseded` | Create a new artifact from scratch with `forgeplan new` |
| `New draft created but still empty` | Expected - reopen only scaffolds | Open the new draft, fill MUST sections, then validate |

## See also

- [`forgeplan renew`](/docs/cli/renew/) - extend validity when the decision is still correct
- [`forgeplan supersede`](/docs/cli/supersede/) - replace with an already-active successor
- [`forgeplan deprecate`](/docs/cli/deprecate/) - retire with no follow-up
- [`forgeplan new`](/docs/cli/new/) - manually scaffold a fresh artifact if reopen doesn't fit
- [`forgeplan activate`](/docs/cli/activate/) - activate the new draft once it's ready
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Methodology: Artifact Lifecycle](/docs/methodology/lifecycle/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan reopen --help`.

```text
Reopen an artifact — creates a NEW draft artifact, deprecates the old one

Usage: forgeplan reopen --reason <REASON> <ID>

Arguments:
  <ID>  Artifact ID to reopen

Options:
      --reason <REASON>  Reason for reopening
  -h, --help             Print help
  -V, --version          Print version
```
<!-- forgeplan-runtime:end -->
