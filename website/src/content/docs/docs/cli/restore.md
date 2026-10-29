---
title: forgeplan restore
description: "Restore a soft-deleted artifact from the most recent non-consumed receipt in `.forgeplan/trash/`. Surgical recovery by explicit ID."
---

`forgeplan restore` brings back a specific artifact that was previously deleted, superseded, or deprecated. You give it the ID (e.g. `PRD-042`); it finds the most recent eligible receipt in `.forgeplan/trash/` and reverses the recorded operation. The lifecycle status returns to the state captured in the receipt. On Constellation, canonical Markdown is restored first and reconciliation publishes the derived index; inspect warnings and `index_pending` rather than assuming publication succeeded. Relations whose targets no longer exist can be skipped.

For delete recovery, a conflicting live identity is a refusal: inspect both
artifacts and the receipt before making any change. Do not automatically
rename, supersede or delete a live artifact to force recovery. A valid,
unconsumed receipt is required; the default retention is 30 days.

This is the precise version of [`forgeplan undo-last`](/docs/cli/undo-last/) - `undo-last` reverses the newest destructive operation regardless of artifact, while `restore` targets one specific artifact by ID. It mirrors [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) on the MCP side, except that the MCP tool additionally accepts an optional `target_status` override; the CLI always restores the status captured in the receipt.

## When to use

- You realised yesterday's `forgeplan delete PRD-042` was wrong - restore by exact ID.
- A `forgeplan supersede` decision was incorrect and you want the original back, not whichever happened to be the newest destructive operation.
- You want precise recovery without risk of reversing an unrelated operation that happened more recently.

## When NOT to use

- You do not remember the ID - [`forgeplan undo-last`](/docs/cli/undo-last/) walks the destructive log newest-first and finds it for you.
- A conflicting live artifact exists - inspect and resolve the identity conflict deliberately before retrying.
- No valid receipt remains - inspect version control history; do not invent a lifecycle transition to bypass receipt recovery.

## Usage

```text
forgeplan restore [OPTIONS] <ID>
```

## Arguments

```text
  <ID>  Artifact ID to recover from the most recent non-consumed receipt
```

## Options

```text
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```

## Examples

### Example 1: Restore a deleted PRD

```bash
forgeplan restore PRD-042
```

Finds the newest unused trash record for `PRD-042`, replays the original operation in reverse, and prints how many links were reconnected. If some links pointed to artifacts that no longer exist, they are reported in `relations_skipped` so you know what to fix by hand.

### Example 2: Verify after restoring

```bash
forgeplan restore PRD-042
forgeplan get PRD-042
```

Always inspect the body, status, and links after a restore - confirm everything looks right. Re-create any links reported as skipped manually if you need them.

### Example 3: Machine-readable output

```bash
forgeplan restore PRD-042 --json | jq '.relations_skipped'
```

Returns JSON and extracts only the list of links that could not be restored. Useful in a recovery script that should attempt to re-create those links from another source.

## How it fits the workflow

Inspect the destructive-operation history and matching receipt, run
`forgeplan restore <ID>`, then inspect `forgeplan get <ID>` and its links.
Recovery reverses a recorded operation; it is not ordinary terminal-status
reactivation. Command availability does not establish migrated-workspace
support for every recovery tool; see [Constellation](/docs/guides/constellation/).

## See also

- [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) - MCP equivalent
- [`forgeplan undo-last`](/docs/cli/undo-last/) - reverse the most recent destructive op (no ID needed)
- [`forgeplan activity`](/docs/cli/activity/) - locate the receipt before restoring
- [`forgeplan delete`](/docs/cli/delete/) - the soft-delete this reverses
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan restore --help`.

```text
Restore a soft-deleted artifact from the most recent non-consumed receipt in `.forgeplan/trash/`. Works for delete (recreates row + moves projection back), supersede (resets status + drops link), and deprecate (resets status). Refuses if a different artifact with the same ID currently exists. TTL default: 30 days from the destructive op

Usage: forgeplan restore [OPTIONS] <ID>

Arguments:
  <ID>  Artifact ID to recover from the most recent non-consumed receipt

Options:
      --json     Output as JSON for machine consumption
  -h, --help     Print help
  -V, --version  Print version
```
<!-- forgeplan-runtime:end -->
