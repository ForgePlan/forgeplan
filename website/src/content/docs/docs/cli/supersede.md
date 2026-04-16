---
title: forgeplan supersede
description: "Replace an active artifact with a newer one — terminal transition that preserves lineage via a --by link."
---

`forgeplan supersede` retires an active artifact and records that a specific replacement takes over its responsibility. The old artifact transitions to `superseded` — a terminal state it can never leave — and a typed link is written from the old ID to the new one. This is the canonical way to evolve decisions in Forgeplan: the history is preserved, back-references keep working, and anyone reading the old artifact is immediately pointed at the current answer.

## When to use

- You rewrote an ADR with a new decision and the newer version is already active (`ADR-005` replaces `ADR-003`).
- A v2 RFC subsumes a v1 RFC: same problem, better architecture, the old plan no longer applies.
- A PRD was split into two more focused PRDs, and one of them carries the continuation of the original scope.
- A Spec was refactored and the old contract is no longer valid but must stay readable for history.

## When NOT to use

- The old artifact simply became obsolete with no direct replacement — use [`forgeplan deprecate`](/docs/cli/deprecate/) with a reason instead.
- You want to re-explore the problem and draft a successor from scratch — use [`forgeplan reopen`](/docs/cli/reopen/), which creates a new draft automatically.
- The replacement artifact does not exist yet. Create and activate it first, then supersede.
- The old artifact is still in `draft`. Drafts don't need superseding — just delete or rewrite them.

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
forgeplan links PRD-007
```

After superseding, inspect the relationship graph to confirm the `superseded_by` edge was written.

## How it fits the workflow

Supersede is the clean exit of the `Shape → Validate → Code → Evidence → Activate` cycle for a decision that must be replaced rather than retired. It is always paired with the activation of the successor: the new artifact goes through the full cycle, becomes `active`, and only then do you run `supersede` on the predecessor. Because the state is terminal, make sure the replacement is really the answer — if you are still exploring, use `reopen`.

## Common errors

| Error | Cause | Fix |
|---|---|---|
| `--by artifact not found` | Replacement ID doesn't exist | Create and activate the successor first |
| `--by artifact is in draft` | Successor not activated yet | Run `forgeplan activate <new-id>` before superseding |
| `Cannot supersede from draft` | Original is still in draft | Drafts don't need superseding — edit or delete |
| `Already in terminal state` | Artifact is already superseded or deprecated | Terminal states are final — nothing to do |

## See also

- [`forgeplan deprecate`](/docs/cli/deprecate/) — retire without a replacement
- [`forgeplan reopen`](/docs/cli/reopen/) — deprecate and start a new draft for re-evaluation
- [`forgeplan activate`](/docs/cli/activate/) — activate the successor before superseding
- [`forgeplan links`](/docs/cli/link/) — inspect the `superseded_by` relationship
- [Lifecycle v2 guide](/docs/guides/lifecycle-v2/)
- [Methodology: Artifact Lifecycle](/docs/methodology/lifecycle/)
