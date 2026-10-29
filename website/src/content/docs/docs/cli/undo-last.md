---
title: forgeplan undo-last
description: "Reverse the most recent destructive operation (delete, supersede, or deprecate) - the undo button for AI agents. Never guesses; errors with guidance when no receipt is found."
---

`forgeplan undo-last` reverses the most recent destructive operation - `delete`, `supersede`, or `deprecate` - without needing to know the artifact ID. Think of it as the "undo" button after an agent (or you) did something wrong: the artifact comes back, its links are restored where the targets still exist, and its status is flipped back from `superseded` / `deprecated`.

How it works: every destructive operation writes a small record (a "receipt") to `.forgeplan/trash/`. `undo-last` finds the newest receipt that has not already been used, replays it in reverse, and marks it consumed so a second call moves to the next-most-recent operation. If no receipt matches your time window, the command errors with guidance - it never guesses.

This is the CLI version of [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) on the MCP side.

## When to use

- Right after a `forgeplan delete` / `supersede` / `deprecate` you regret - undo, then redo correctly.
- User authorizes undo without an ID - inspect the receipt candidate before reversing an unrelated operation.
- Reversing an LLM hallucination that triggered a destructive action.
- After inspecting a destructive call in [`forgeplan activity`](/docs/cli/activity/) and identifying its recovery receipt.

## When NOT to use

- You know the exact artifact ID to restore - [`forgeplan restore <ID>`](/docs/cli/restore/) is more precise (no chance of reversing the wrong operation).
- Beyond the default 30-day receipt TTL - recovery may be refused; use preserved history or backups.
- A typo/title change is not a destructive receipt operation - use `update`, then appropriate `index`/`reindex` if needed.

## Usage

```text
forgeplan undo-last [OPTIONS]
```

## Options

```text
      --within-hours <WITHIN_HOURS>  Time window (hours) to search for the last destructive op (1..=720, default 24) [default: 24]
      --json                         Output as JSON for machine consumption
  -h, --help                         Print help
  -V, --version                      Print version
```

## Examples

### Example 1: Default 24-hour undo

```bash
forgeplan undo-last
```

Reverses the most recent destructive operation from the last 24 hours. Each call consumes one receipt - so calling it three times in a row undoes the last three operations in reverse order (newest first).

### Example 2: Wider search after a pause

```bash
forgeplan undo-last --within-hours 720
```

Searches the full 30-day window. Use this when you come back to a workspace after several days and the default 24-hour search returns nothing.

### Example 3: Machine-readable output for scripts

```bash
forgeplan undo-last --json | jq '.restored, .op_reversed'
```

Returns JSON, then extracts the restored artifact ID and the type of operation that was reversed. Useful when `undo-last` is part of a recovery script that needs to log what it actually did.

On migrated layouts inspect `index_pending`, `warnings` and `relations_skipped`.
A successful canonical restore does not guarantee immediate derived publication.

## How it fits the workflow

Use after a destructive operation goes wrong (`delete`, `supersede`, or `deprecate`). Run `undo-last` to reverse the most recent one; repeat the call to undo earlier ones in order. Once you know the specific ID, switch to [`forgeplan restore <ID>`](/docs/cli/restore/) - it targets a single artifact instead of walking the stack newest-first.

## See also

- [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) - MCP equivalent
- [`forgeplan restore`](/docs/cli/restore/) - restore a specific artifact by ID
- [`forgeplan activity`](/docs/cli/activity/) - inspect the destructive-op timeline
- [`forgeplan delete`](/docs/cli/delete/) - the soft-delete this reverses
- [CLI overview](/docs/cli/)

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan undo-last --help`.

```text
Reverse the most recent destructive operation (delete, supersede, or deprecate) by reading the soft-delete trash and applying restore to the most recently written non-consumed receipt. If no matching receipt is found, returns an error with guidance; the tool never guesses

Usage: forgeplan undo-last [OPTIONS]

Options:
      --within-hours <WITHIN_HOURS>  Time window (hours) to search for the last destructive op (1..=720, default 24) [default: 24]
      --json                         Output as JSON for machine consumption
  -h, --help                         Print help
  -V, --version                      Print version
```
<!-- forgeplan-runtime:end -->
