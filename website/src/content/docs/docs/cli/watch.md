---
title: forgeplan watch
description: "Watch canonical files and maintain the owning derived index."
---

Watch canonical Markdown and update the derived index after change batches.
For the local `0.39.0+constellation.s1.1` build, a migrated workspace requires
`forgeplan watch --constellation`; plain `watch` refuses there.

## Usage

```text
forgeplan watch [OPTIONS]
```

## Options

```text
      --constellation  Run Constellation maintenance after change batches
  -h, --help     Print help
  -V, --version  Print version
```

## How it works

The watcher batches filesystem changes before reconciling derived state.
In Constellation mode it publishes index maintenance results rather than
using the legacy sync path for a migrated workspace. Reads can observe fresh
canonical files even before a new generation is persisted.

The captured help's “alongside the legacy sync” description is not the migrated path: there the legacy store is absent and skipped. No retired artifact tables are recreated.

Watching does not by itself configure embeddings. Automatic queue draining
requires explicit `embedding:` configuration, a semantic-search build,
`bge-m3`, and its complete cached snapshot. A manual `embed` does not save
that configuration. See [Constellation setup](/docs/guides/constellation/).

## When to use it

- **Interactive writing** - maintain the owning index after save batches; migrated reads can see canonical changes before publication.
- **AI-assisted editing** - let maintenance reconcile canonical changes rather than assuming every save immediately persists a new generation.
- **Reorganization** - Constellation reconciliation tracks the canonical view. The legacy deleted-file path only warns; it does not automatically remove deleted artifact rows.

## When NOT to use it

- **One-shot rebuilds** - prefer [`index`](/docs/cli/index-command/) for Constellation; use legacy [`reindex`](/docs/cli/reindex/) where applicable. `scan-import` imports documents, not routine index recovery.
- **CI / scripted workflows** - a one-shot rebuild avoids a long-running watcher.

## Example

Illustrative commands, not a captured runtime transcript:

```bash
# Terminal 1, migrated workspace
forgeplan watch --constellation

# Terminal 2, after saving an artifact
forgeplan search "auth" --keyword
```

Stop the watcher with `Ctrl-C`.

## Alternatives

- **Manual** - use [`index`](/docs/cli/index-command/) for Constellation after edit batches.
- **Full MCP host** - [`serve --auto-index`](/docs/cli/serve/) can maintain its configured workspace with the appropriate build, embedding configuration and cached model; a read-only profile is not a maintenance host.

## Limitations

- Not a background service - runs in the foreground, so pair it with `tmux`,
  a terminal pane, or your shell's job control if you want it persistent.
- No remote filesystem support - relies on inotify/FSEvents/kqueue (all
  native OS watchers), so network mounts may miss events.
- Single-workspace - run one `watch` per `.forgeplan/` root.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan reindex`](/docs/cli/reindex/) - one-shot manual rebuild
- [`forgeplan git-sync`](/docs/cli/git-sync/) - incremental sync after git pull
- [`forgeplan serve`](/docs/cli/serve/) - MCP server for AI agents

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan watch --help`.

```text
Watch .forgeplan/ files and sync changes to LanceDB in real time

Usage: forgeplan watch [OPTIONS]

Options:
      --constellation  Also run a Constellation maintenance pass after each change batch: the corrected 0A/0B index stays fresh alongside the legacy sync
  -h, --help           Print help
  -V, --version        Print version
```
<!-- forgeplan-runtime:end -->
