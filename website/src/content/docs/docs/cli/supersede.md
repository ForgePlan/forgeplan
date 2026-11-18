---
title: forgeplan supersede
description: "Replace an active artifact with a newer one - terminal transition that preserves lineage via a --by link."
---

`forgeplan supersede` retires an active artifact and records a replacement. It writes a `supersedes` relation from the old ID to the replacement and changes the old status to `superseded`. Ordinary lifecycle transitions cannot leave that state; valid receipt-based recovery is a separate mechanism. The historical artifact remains readable.

## When to use

- You rewrote an ADR with a new decision and the newer version is already active (`ADR-005` replaces `ADR-003`).
- A v2 RFC subsumes a v1 RFC: same problem, better architecture, the old plan no longer applies.
- A PRD was split into two more focused PRDs, and one of them carries the continuation of the original scope.
- A Spec was refactored and the old contract is no longer valid but must stay readable for history.
- An **EvidencePack** no longer applies after re-verification: link the justified successor and retire the displaced pack. Terminal packs leave eligible R_eff populations. Recompute and inspect affected scores; immediate ancestor-cache refresh is not guaranteed on every storage path.

## When NOT to use

- The old artifact simply became obsolete with no direct replacement - use [`forgeplan deprecate`](/docs/cli/deprecate/) with a reason instead.
- You want to re-explore the problem and draft a successor from scratch - use [`forgeplan reopen`](/docs/cli/reopen/), which creates a new draft automatically.
- The replacement artifact does not exist yet. Create and activate it first, then supersede.
- The old artifact is still in `draft`. Drafts don't need superseding - just delete or rewrite them.

## Usage

```text
forgeplan supersede --by <BY> <ID>
```

## Arguments

```text
  <ID>  Artifact ID to supersede
```

## Options

```text
      --by <BY>  Replacement artifact ID
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: Replace ADR-003 with ADR-005

```bash
forgeplan activate ADR-005
forgeplan supersede ADR-003 --by ADR-005
```

Activate the successor first so the replacement link points at a live decision, then supersede the original.

### Example 2: Roll an RFC from v1 to v2

```bash
forgeplan supersede RFC-002 --by RFC-014
```

`RFC-002` enters `superseded` (terminal). Anyone who follows a stale link to `RFC-002` sees the `--by` pointer to `RFC-014`.

### Example 3: Verify the lineage

```bash
forgeplan supersede PRD-007 --by PRD-021
forgeplan graph
```

Inspect the graph or `forgeplan get PRD-007 --json` for the `supersedes` edge from the old artifact to its replacement. There is no `forgeplan links` command.

## How it fits the workflow

Supersede is the clean exit of the `Shape → Validate → Code → Evidence → Activate` cycle for a decision that must be replaced rather than retired. It is always paired with the activation of the successor: the new artifact goes through the full cycle, becomes `active`, and only then do you run `supersede` on the predecessor. Because the state is terminal, make sure the replacement is really the answer - if you are still exploring, use `reopen`.

## Common errors

| Error | Cause | Fix |
|---|---|---|
| `--by artifact not found` | Replacement ID doesn't exist | Create and activate the successor first |
| Replacement is draft (warning) | Successor not activated yet | The command warns rather than refusing solely for draft status; activate a justified successor first as workflow policy |
| `Cannot supersede from draft` | Original is still in draft | Drafts don't need superseding - edit or delete |
| Already terminal | Artifact is superseded or deprecated | Ordinary transition is refused; inspect receipt-based recovery separately |

## See also

- [`forgeplan deprecate`](/docs/cli/deprecate/) - retire without a replacement
- [`forgeplan reopen`](/docs/cli/reopen/) - deprecate and start a new draft for re-evaluation
- [`forgeplan activate`](/docs/cli/activate/) - activate the successor before superseding
- [`forgeplan graph`](/docs/cli/graph/) - inspect the recorded `supersedes` relationship
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Methodology: Artifact Lifecycle](/docs/methodology/lifecycle/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan supersede --help`.

```text
Supersede an artifact (active → superseded) with replacement link

Usage: forgeplan supersede --by <BY> <ID>

Arguments:
  <ID>  Artifact ID to supersede

Options:
      --by <BY>  Replacement artifact ID
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
