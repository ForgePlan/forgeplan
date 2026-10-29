---
title: forgeplan git-sync
description: "Sync artifact changes from git operations (pull/merge) into LanceDB"
---

Sync artifact state after a Git operation. On a legacy workspace the command selects committed changes between a ref and HEAD and synchronizes the corresponding files. On an already Constellation-managed workspace it runs canonical index reconciliation instead; ordinary `git-sync` does not migrate a legacy workspace by itself.

## Usage

```text
forgeplan git-sync [OPTIONS]
```

## Options

```text
      --since <SINCE>  Git ref to diff against (default: ORIG_HEAD from last pull/merge)
  -h, --help           Print help
  -V, --version        Print version
```

## How it works

1. Select the ref (`--since` or `ORIG_HEAD`) and inspect committed `.forgeplan/` changes.
2. On a migrated workspace, reconcile the canonical view through Constellation.
3. On a legacy workspace, parse changed artifact files, synchronize rows, and remove rows for deleted files.

Do not infer universal ancestor-score reconciliation from this pass.

`ORIG_HEAD` is the default because `git pull`/`git merge`/`git rebase` all
set it to the previous tip of the current branch, giving `git-sync` an exact
"what this pull brought in" window.

## When to use it

- **After `git pull`** - make pulled artifact changes visible in the workspace's owning index. Runtime cost depends on layout and workspace size; no subsecond result is guaranteed.
- **After `git merge feature/xyz`** - same idea for local merges.
- **After `git checkout`** between branches with divergent artifact state -
  pass `--since <other-branch>` to sync the diff.

## Remote team workflow

```bash
# Morning routine with an active team
git checkout dev
git pull origin dev          # pulls 3 new PRDs from teammates
forgeplan git-sync           # synchronize using this workspace's owning layout
forgeplan list              # inspect the current artifact listing
forgeplan health             # check for new blind spots
```

Compare against a cold rebuild:

```bash
forgeplan reindex            # legacy rebuild, or Constellation reconcile on owned layouts
```

Legacy selection can reduce the files synchronized. A migrated workspace uses a reconcile pass, so do not apply the same incremental-cost estimate to both layouts.

## `git-sync` vs `reindex` vs `watch`

| Command | Use when | Layout distinction |
|---|---|---|
| `git-sync` | After a Git operation with a known base | Legacy selected-file sync; migrated canonical reconcile |
| `index` | Maintain Constellation explicitly | Can migrate a legacy workspace |
| `reindex` | Rebuild derived state from canonical files | Legacy rebuild; selected build routes Constellation-owned layouts to reconcile |
| `watch` | Maintain after save batches | Migrated workspace requires `--constellation` |

For Constellation recovery prefer explicit `forgeplan index`; do not recreate retired tables or use `scan-import` as an index-repair command.

## Example

```bash
# Default - since last pull/merge
forgeplan git-sync

# Explicit diff base (e.g. against main)
forgeplan git-sync --since origin/main

# Check what a PR branch brought in
git checkout feat/prd-050-discover
forgeplan git-sync --since dev
```

## Limitations

- The legacy selection is a committed ref-to-HEAD comparison, not an uncommitted-edit detector. Use the appropriate watcher or explicit rebuild after manual edits.
- If `ORIG_HEAD` is stale, pass `--since` explicitly.
- Migrated reconciliation reads the canonical view, so the legacy selected-file limitation is not the migrated read contract.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan reindex`](/docs/cli/reindex/) - full rebuild fallback
- [`forgeplan watch`](/docs/cli/watch/) - live sync daemon
- [`forgeplan health`](/docs/cli/health/) - verify after sync

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan git-sync --help`.

```text
Sync artifact changes from git operations (pull/merge) into LanceDB

Usage: forgeplan git-sync [OPTIONS]

Options:
      --since <SINCE>  Git ref to diff against (default: ORIG_HEAD from last pull/merge)
  -h, --help           Print help
  -V, --version        Print version
```
<!-- forgeplan-runtime:end -->
