---
title: forgeplan restore
description: "Recover an artifact from an existing non-consumed undo receipt. CLI delete captures receipts; CLI supersede/deprecate do not. Inspect stored state and Markdown separately."
---

`forgeplan restore` requires an existing undo receipt in `.forgeplan/trash/`; an artifact ID alone cannot reconstruct a past operation. It finds the newest matching non-consumed receipt and applies its captured state. Supported receipt types are delete (recreate the row and attempt to move the Markdown projection back), supersede (reset stored status/title and drop the old → replacement link), and deprecate (reset stored status/title). Captured relations are re-added where the other artifact still exists.

**Receipt prerequisite in 0.38.0:** CLI `forgeplan delete` captures a receipt before deletion. CLI `forgeplan supersede` and `forgeplan deprecate` do **not** capture receipts; the corresponding MCP tools do. Restore can consume those MCP-created receipts, but it cannot undo a CLI status change merely because that operation type is supported. Check that the receipt belongs to the intended operation rather than assuming an older receipt for the same ID represents it.

For a delete receipt, any existing record with that ID blocks restore. Superseding it does not free the ID. For supersede/deprecate receipts the row normally still exists: restore resets its captured status/title and, for supersede, drops the old → replacement link; a different kind blocks recovery. Preserve both versions and compare them before resolving a collision manually. Do not delete or supersede live work just to make restore pass.

The CLI starts a best-effort asynchronous purge of receipts and paired bodies older than 30 days. This is lazy cleanup, not guaranteed deletion at an exact deadline, and receipt lookup itself has no age filter. Treat trash as short-term recovery, not a backup.

This is the precise version of [`forgeplan undo-last`](/docs/cli/undo-last/) - `undo-last` reverses the newest destructive operation regardless of artifact, while `restore` targets one specific artifact by ID. Mirrors [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) on the MCP side.

## When to use

- You realised yesterday's `forgeplan delete PRD-042` was wrong - restore by exact ID.
- An existing supersede/deprecate receipt, such as one captured by the corresponding MCP tool, represents the operation you want to reverse.
- You want precise recovery without risk of reversing an unrelated operation that happened more recently.

## When NOT to use

- You do not remember the ID - [`forgeplan undo-last`](/docs/cli/undo-last/) walks the destructive log newest-first and finds it for you.
- A delete receipt collides with a live ID - inspect and preserve both versions before choosing a recovery plan.
- No suitable receipt remains, including a CLI supersede/deprecate that never captured one - preserve current state and consult the original projections/version-control history or a backup before deciding how to recover. `reindex` can import a recovered projection when its row is absent, but does not refresh existing status/title/depth/tags or remove obsolete links between surviving records. Do not delete the index or use `scan-import` as a metadata repair.

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
forgeplan get PRD-042 --json
```

Inspect the stored body, status, and links through `get --json`, **and separately open the actual `.forgeplan/` Markdown projection** to compare its frontmatter, body, and links. `get` reads the store and does not establish Markdown/index agreement. Re-create skipped relations only after checking their original direction/type and whether the other artifact should exist.

### Example 3: Machine-readable output

```bash
forgeplan restore PRD-042 --json | jq '.relations_skipped'
```

Returns JSON and extracts the IDs at the other end of relations skipped because those artifacts are missing. It is not a list of complete relation triples. Inspect the receipt to recover direction and relation type before re-linking.

Success JSON has `ok`, `restored`, `op_reversed`, `relations_restored`, `relations_skipped`, `projection_restored`, `warnings`, and `_next_action`. No-receipt JSON has `ok: false`, `error`, and `_next_action` and exits 1. Other restore errors are ordinary CLI errors, not necessarily this JSON shape. Success can include warnings or `projection_restored: false`; inspect both rather than assuming the file returned. Restored projections/metadata are not a full historical byte-for-byte rollback.

**Known projection mismatch ([#296 ARCH-5](https://github.com/ForgePlan/forgeplan/issues/296#issuecomment-6374114519)):** for supersede/deprecate receipts, the released helper sets `projection_restored: true` because the file was never moved; it does not re-render it after resetting the store. Stored status can be `active` while Markdown still says `deprecated` or `superseded`. Thus even `ok: true` plus `projection_restored: true` is not proof that both representations agree. Preserve and inspect both; neither `get` alone nor a generic `reindex` is a complete repair for this mismatch.

## How it fits the workflow

Identify the intended receipt and compare it with the current artifact before restoring. Activity logs can help establish the timeline but are not undo receipts. After `restore <ID>`, inspect both stored state (`get --json`) and the actual projection file, including status and relations. Review warnings and skipped peers before any manual repair; successful response flags do not close the known projection mismatch.

## See also

- [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) - MCP equivalent
- [`forgeplan undo-last`](/docs/cli/undo-last/) - reverse the most recent destructive op (no ID needed)
- [`forgeplan activity`](/docs/cli/activity/) - locate the receipt before restoring
- [`forgeplan delete`](/docs/cli/delete/) - the soft-delete this reverses
- [CLI overview](/docs/cli/)
