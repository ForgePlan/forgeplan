---
title: forgeplan undo-last
description: "Reverse the most recent destructive operation (delete, supersede, or deprecate) - the undo button for AI agents. Never guesses; errors with guidance when no receipt is found."
---

`forgeplan undo-last` restores the newest non-consumed destructive-operation
receipt within the requested time window, regardless of artifact ID.
It can replay delete, supersede, and deprecate receipts, but it does not
infer operations from the change log or Git history.

Receipt coverage matters: CLI 0.38.0 `delete` captures a receipt, whereas CLI
`supersede` and `deprecate` do not. Their MCP handlers capture receipts.
Thus “undo last” means the last **available receipt**, not necessarily the
last destructive action you performed. Inspect the relevant action and
artifact before running it; the command has no preview or confirmation prompt.

This is the CLI version of [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) on the MCP side.

## When to use

- After a mistaken receipt-backed delete, or a receipt-backed MCP supersede/deprecate.
- After identifying the intended operation from its artifact and receipt context.
- In a recovery script that checks the returned artifact ID and warnings.

## When NOT to use

- You know the exact artifact ID to restore - [`forgeplan restore <ID>`](/docs/cli/restore/) is more precise (no chance of reversing the wrong operation).
- Outside the maximum 720-hour selection window, or after the receipt/body was purged — use version-control recovery. The default receipt retention is 30 days, with opportunistic purge rather than a guaranteed timed deletion.
- The mistake was a typo or wrong title (not a destructive operation) - edit the file directly, then run `forgeplan reindex`.

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

Selects the newest available non-consumed receipt within 24 hours. A successful restore marks it consumed; repeated calls move to older receipts. A failed restore may leave earlier mutations applied and the receipt unconsumed — inspect state before retrying.

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

Success JSON contains `ok: true`, `restored` (ID), `op_reversed`,
`receipt_id`, `relations_restored` (count), `relations_skipped` (IDs),
`projection_restored` (boolean), `warnings`, and `_next_action`.
This CLI does not emit `restored_status`. No candidate exits `1` with
`ok: false`, `error`, and `_next_action`; other restore failures can use stderr.
The supplied unsigned hour count is clamped to 1–720.

For delete, the row is recreated and a trashed projection is moved back when
available; an existing row with the same ID is refused. For supersede/deprecate
receipts, an existing same-kind row has its status/title reset, and supersede's
replacement edge is removed. This does not rewind every subsequent body edit
or regenerate its projection. Missing relation endpoints are skipped and
reported, not invented. Inspect warnings and both file/store state after recovery.

## How it fits the workflow

Use this only when the newest available receipt is the operation you intend to reverse. Once the ID is known, [`forgeplan restore <ID>`](/docs/cli/restore/) selects recovery for that artifact instead of walking workspace receipts newest-first.

## See also

- [`forgeplan_undo_last`](/docs/mcp/forgeplan_undo_last/) - MCP equivalent
- [`forgeplan restore`](/docs/cli/restore/) - restore a specific artifact by ID
- [`forgeplan activity`](/docs/cli/activity/) - inspect the destructive-op timeline
- [`forgeplan delete`](/docs/cli/delete/) - the soft-delete this reverses
- [CLI overview](/docs/cli/)
