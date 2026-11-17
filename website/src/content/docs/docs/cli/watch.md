---
title: forgeplan watch
description: "Watch Markdown files directly inside artifact directories and debounce body synchronization to LanceDB; deletions only warn."
---

Start a foreground watcher for Markdown files directly inside existing
artifact directories under `.forgeplan/`. It is useful when switching
between an editor and the CLI: processed body changes reach LanceDB without
a manual `reindex`, after a debounce delay rather than immediately.

## Usage

```text
forgeplan watch
```

## Options

```text
  -h, --help     Print help
  -V, --version  Print version
```

## How it works

1. Watches each existing artifact directory non-recursively (not arbitrary `.forgeplan/**/*.md`).
2. Debounces events for 500 ms and deduplicates paths within each batch.
3. For existing records, uses the file-to-store projection sync helper; new files need an `id` in frontmatter and create records.
4. Prints `NEW`/`SYNC` messages on stdout and errors on stderr.
5. Stops on `Ctrl-C`, printing `Shutting down watcher.` It does not promise to drain pending events before exit.

Deletion events only warn; they do not remove indexed records. This is not a complete metadata/link/scoring rebuild or an embedding refresh. `reindex` scans workspace projections, imports new records from frontmatter, updates differing bodies on existing records, adds missing frontmatter links, and removes orphan records/relations. It does not refresh existing title/depth/status/tags or remove deleted links between surviving records; use supported explicit mutation commands such as `update --depth` for those changes.

## When to use it

- **Interactive writing sessions** - you're drafting a PRD in VS Code and
  want `forgeplan list` / `forgeplan search` to reflect each save.
- **Pair work with AI agents** - agent edits markdown, watcher pushes into
  LanceDB, next `search` call sees the update without a round-trip reindex.
- **Occasional saves** - keep the watcher visible and wait for its sync message before querying. After moving/renaming projections, use `reindex` for path discovery and orphan cleanup, while accounting for its existing-record metadata/link limits. Arbitrary nested folders are not added to the watch set.

## When NOT to use it

- **Batch imports** - use `reindex` to scan workspace projections, or `scan-import` to import external Markdown. `scan-import` skips already imported artifacts; it is not an existing-record refresh.
- **CI / scripted body synchronization** - one-shot `reindex` avoids a long-running watcher but still has the metadata/link limits above.
- **Post-`git pull`** - use [`git-sync`](/docs/cli/git-sync/), which knows
  exactly what changed from the git diff.

## Example

```bash
# Terminal 1: leave this foreground process running.
forgeplan watch

# Terminal 2: edit an existing artifact's actual projection file.
# Wait for the sync message, then query the stored content.
forgeplan search "auth"
```

Illustrative output for one existing artifact (not an executed run):

```text
SYNC PRD-001 — body updated from file
```

Stop with `Ctrl-C`:

```text
^C
Shutting down watcher.
```

## Alternatives

- **Manual** - run [`forgeplan reindex`](/docs/cli/reindex/) after a batch of body edits or to import new workspace projections. It is not a full frontmatter/link reset for existing records.
- **Post-pull** - [`forgeplan git-sync`](/docs/cli/git-sync/) diffs against
  `ORIG_HEAD` and only re-indexes files the merge/pull actually touched.

## Limitations

- Not a background service - runs in the foreground, so pair it with `tmux`,
  a terminal pane, or your shell's job control if you want it persistent.
- No remote filesystem support - relies on inotify/FSEvents/kqueue (all
  native OS watchers), so network mounts may miss events.
- Single-workspace - run one `watch` per `.forgeplan/` root.
- Errors are logged per event and do not stop the loop; a running watcher is not proof that every save was indexed.
- File deletion leaves the record in LanceDB. Review intent before using an explicit lifecycle/delete command; do not assume a rename removed the old record.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan reindex`](/docs/cli/reindex/) - projection scan, body sync, and orphan cleanup
- [`forgeplan git-sync`](/docs/cli/git-sync/) - incremental sync after git pull
- [`forgeplan serve`](/docs/cli/serve/) - MCP server for AI agents
