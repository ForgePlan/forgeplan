---
title: forgeplan restore
description: "Restore a soft-deleted artifact from the most recent non-consumed receipt in `.forgeplan/trash/`. Surgical recovery by explicit ID."
---

`forgeplan restore` brings back a specific artifact that was previously deleted, superseded, or deprecated. You give it the ID (e.g. `PRD-042`); it finds the most recent record of that artifact's destruction in `.forgeplan/trash/` and replays it in reverse: the markdown file comes back, the search index entry is recreated, links are restored where their targets still exist, and the lifecycle status is flipped back from `superseded` / `deprecated`.

If a different artifact already exists with the same ID, the command refuses (you must resolve the conflict manually first — rename or supersede the live one, then retry). Records older than 30 days are deleted and cannot be restored.

This is the precise version of [`forgeplan undo-last`](/docs/cli/undo-last/) — `undo-last` reverses the newest destructive operation regardless of artifact, while `restore` targets one specific artifact by ID. Mirrors [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) on the MCP side.

## When to use

- You realised yesterday's `forgeplan delete PRD-042` was wrong — restore by exact ID.
- A `forgeplan supersede` decision was incorrect and you want the original back, not whichever happened to be the newest destructive operation.
- You want precise recovery without risk of reversing an unrelated operation that happened more recently.

## When NOT to use

- You do not remember the ID — [`forgeplan undo-last`](/docs/cli/undo-last/) walks the destructive log newest-first and finds it for you.
- A different artifact with the same ID currently exists — resolve the conflict first (rename or supersede the live one), then run `restore`.
- The destruction was more than 30 days ago — the trash record has been purged. Recover by reading the artifact body from `git log` and re-creating it manually.

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
forgeplan show PRD-042
```

Always inspect the body, status, and links after a restore — confirm everything looks right. Re-create any links reported as skipped manually if you need them.

### Example 3: Machine-readable output

```bash
forgeplan restore PRD-042 --json | jq '.relations_skipped'
```

Returns JSON and extracts only the list of links that could not be restored. Useful in a recovery script that should attempt to re-create those links from another source.

## How it fits the workflow

Recovery is two steps: figure out when the destruction happened, then restore. Use [`forgeplan activity`](/docs/cli/activity/) with a destructive-operation filter to confirm the timeline, run `forgeplan restore <ID>`, then verify with `forgeplan show <ID>`. Re-link any skipped relations by hand if they matter.

## See also

- [`forgeplan_restore`](/docs/mcp/forgeplan_restore/) — MCP equivalent
- [`forgeplan undo-last`](/docs/cli/undo-last/) — reverse the most recent destructive op (no ID needed)
- [`forgeplan activity`](/docs/cli/activity/) — locate the receipt before restoring
- [`forgeplan delete`](/docs/cli/delete/) — the soft-delete this reverses
- [CLI overview](/docs/cli/)
