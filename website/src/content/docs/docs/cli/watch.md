---
title: forgeplan watch
description: "Watch .forgeplan/ files and sync changes to LanceDB in real time"
---

Start a file-watcher daemon that observes `.forgeplan/**/*.md` and re-indexes
any artifact that changes on disk. Ideal for interactive sessions where you
flip between a markdown editor and the CLI — edits become searchable
immediately without manual `reindex` calls.

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

1. Opens a recursive filesystem watcher rooted at `.forgeplan/`.
2. On every `CREATE`/`MODIFY`/`DELETE` event for a `.md` file under
   tracked artifact directories, schedules a debounced re-parse.
3. Re-parses the changed file, updates LanceDB in place, re-computes
   derived fields (tags, links, scoring).
4. Logs each sync event to stdout so you can see what was picked up.
5. Runs until you send `Ctrl-C` (SIGINT) — then exits cleanly.

Debouncing coalesces rapid-fire events (e.g. an editor writing a temp file
then renaming over the target), so one save = one index update.

## When to use it

- **Interactive writing sessions** — you're drafting a PRD in VS Code and
  want `forgeplan list` / `forgeplan search` to reflect each save.
- **Pair work with AI agents** — agent edits markdown, watcher pushes into
  LanceDB, next `search` call sees the update without a round-trip reindex.
- **Bulk reorganization** — moving/renaming artifacts in a file manager and
  wanting the index to track along.

## When NOT to use it

- **Batch imports** — prefer `reindex` or `scan-import` for one-shot ingestion.
- **CI / scripted workflows** — one-shot `reindex` is simpler and deterministic.
- **Post-`git pull`** — use [`git-sync`](/docs/cli/git-sync/), which knows
  exactly what changed from the git diff.

## Example

```bash
# Start the watcher in one terminal
$ forgeplan watch
[watch] observing .forgeplan/ (Ctrl-C to stop)
[watch] synced prds/prd-001-auth.md (1 artifact)
[watch] synced evidence/evid-042-benchmark.md (1 artifact)

# In another terminal, edit and save — the daemon picks it up
$ vim .forgeplan/prds/prd-001-auth.md
$ forgeplan search "auth"   # sees the new content immediately
```

Stop with `Ctrl-C`:

```text
^C
[watch] shutting down, flushing pending syncs
```

## Alternatives

- **Manual** — run [`forgeplan reindex`](/docs/cli/reindex/) after a batch of
  edits. Simpler if you only save occasionally.
- **Post-pull** — [`forgeplan git-sync`](/docs/cli/git-sync/) diffs against
  `ORIG_HEAD` and only re-indexes files the merge/pull actually touched.

## Limitations

- Not a background service — runs in the foreground, so pair it with `tmux`,
  a terminal pane, or your shell's job control if you want it persistent.
- No remote filesystem support — relies on inotify/FSEvents/kqueue (all
  native OS watchers), so network mounts may miss events.
- Single-workspace — run one `watch` per `.forgeplan/` root.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan reindex`](/docs/cli/reindex/) — one-shot manual rebuild
- [`forgeplan git-sync`](/docs/cli/git-sync/) — incremental sync after git pull
- [`forgeplan serve`](/docs/cli/serve/) — MCP server for AI agents
