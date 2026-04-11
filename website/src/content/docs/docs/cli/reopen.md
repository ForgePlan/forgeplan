---
title: forgeplan reopen
description: "Deprecate an artifact AND create a new draft successor — lineage-preserving re-evaluation of a past decision."
---

`forgeplan reopen` is the "let's think about this again" transition. It deprecates the current artifact (moving it into the terminal `deprecated` state with the reason you supply) and simultaneously creates a new `draft` artifact of the same kind, linked back to the original as a lineage pointer. Use it when a decision has aged out and needs a fresh pass through Shape → Validate → ADI rather than a quick extension. Unlike `supersede`, you are not replacing with an already-active successor — you are starting the exploration over.

## When to use

- An ADR went stale and a review concluded the approach itself needs rethinking, not just a validity extension.
- A PRD's original assumptions no longer match reality and you want to rewrite it rather than patch it.
- A RefreshReport flagged multiple red signals on a previously active decision — reopen to start a fresh draft.
- A ProblemCard you thought was solved has resurfaced in a new form and you need a new artifact to re-scope it.

## When NOT to use

- The decision is still valid and just needs a fresh expiry — use [`forgeplan renew`](/docs/cli/renew/).
- You already have an active replacement — use [`forgeplan supersede`](/docs/cli/supersede/) with `--by`.
- The decision is simply retired with nothing to take its place — use [`forgeplan deprecate`](/docs/cli/deprecate/).
- You need a small amendment — edit the artifact directly or create a follow-up artifact without disturbing the current one.

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
forgeplan refresh ADR-012
forgeplan reopen ADR-012 --reason "refresh report identified 3 invalidated invariants"
```

Combine with `refresh` when the re-evaluation needs a structured report before the new draft is opened.

## How it fits the workflow

Reopen is the most expensive lifecycle transition: it restarts the `Shape → Validate → ADI → Code → Evidence → Activate` cycle for an entire decision. After reopening, you should treat the new draft as a full-depth artifact — fill in MUST sections, run `forgeplan reason` if the depth warrants it, produce fresh evidence, and only then activate. The old artifact survives in terminal state so links from historical context (other artifacts, git history, prior discussions) still resolve. Reach for it when a simple `renew` would hide a real change, and when there isn't yet a successor to `supersede` with.

## Common errors

| Error | Cause | Fix |
|---|---|---|
| `--reason is required` | Flag omitted | Pass `--reason "..."` explaining why re-evaluation is needed |
| `Cannot reopen from draft` | Original never activated | Edit the existing draft directly instead |
| `Cannot reopen from terminal state` | Already `deprecated` or `superseded` | Create a new artifact from scratch with `forgeplan new` |
| `New draft created but still empty` | Expected — reopen only scaffolds | Open the new draft, fill MUST sections, then validate |

## See also

- [`forgeplan renew`](/docs/cli/renew/) — extend validity when the decision is still correct
- [`forgeplan supersede`](/docs/cli/supersede/) — replace with an already-active successor
- [`forgeplan deprecate`](/docs/cli/deprecate/) — retire with no follow-up
- [`forgeplan new`](/docs/cli/new/) — manually scaffold a fresh artifact if reopen doesn't fit
- [`forgeplan activate`](/docs/cli/activate/) — activate the new draft once it's ready
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Methodology: Artifact Lifecycle](/docs/methodology/lifecycle/)
