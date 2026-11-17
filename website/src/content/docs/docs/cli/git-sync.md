---
title: forgeplan git-sync
description: "Sync artifact changes from git operations (pull/merge) into LanceDB"
---

Incrementally sync LanceDB with markdown changes introduced by a git
operation - typically `git pull`, `git merge`, or `git rebase`. Instead of
rebuilding the entire index, `git-sync` selects paths changed between a
git reference and `HEAD`, then reads those paths from disk.

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

1. Runs `git diff --name-status <SINCE> HEAD -- .forgeplan/` and keeps `.md` paths.
2. For added/modified files, reads current disk content and parses frontmatter.
3. Creates new records from frontmatter. Existing records are updated only when their trimmed bodies differ; status is then synchronized if present. Metadata-only changes are skipped, and this path does not comprehensively refresh title, depth, tags, or scores.
4. For deleted files, derives an ID from the filename and removes the matching indexed artifact and its relations.
5. Adds frontmatter links that are not already present for processed files; it does not remove obsolete links. Logs changes with source `git_sync` and the current commit hash.

`ORIG_HEAD` is the default because `git pull`/`git merge`/`git rebase` all
set it to the previous tip of the current branch, giving `git-sync` an exact
"what this pull brought in" window.

## When to use it

- **After `git pull`** - teammates merged new PRDs/RFCs into `dev`, you just
  pulled, and you want them in your local LanceDB. `git-sync` runs in a
  fraction of the time of a full reindex.
- **After `git merge feature/xyz`** - same idea for local merges.
- **After `git checkout`** between branches with divergent artifact state -
  pass `--since <other-branch>` to sync the diff.

## Remote team workflow

```bash
# Morning routine with an active team
git checkout dev
git pull origin dev          # pulls 3 new PRDs from teammates
forgeplan git-sync           # incrementally sync paths changed by the pull
forgeplan log                 # see what arrived
forgeplan health             # check for new blind spots
```

Compare with a workspace-wide projection scan:

```bash
forgeplan reindex            # scans workspace projections; see limits below
```

For small workspaces the difference is negligible; for workspaces with
hundreds of artifacts `git-sync` is substantially faster.

## `git-sync` vs `reindex` vs `watch`

| Command     | Use when                                         | Cost              |
|-------------|--------------------------------------------------|-------------------|
| `git-sync`  | After git pull/merge, know the ref to diff from  | O(changed files)  |
| `reindex`   | New workspace projections, body changes, orphan cleanup | O(all artifacts) |
| `watch`     | Interactive body editing, want debounced sync    | foreground watcher |

**Rule of thumb:** after git moves `HEAD`, `git-sync` can process its changed paths. For manual body changes, use `reindex` or keep `watch` running. `reindex` scans projections, imports new records from their frontmatter, updates differing bodies on existing records, adds missing frontmatter links, and removes orphan records/relations. It is **not** a complete reset: existing title, depth, status, and tags are not refreshed from frontmatter, and removed links between surviving records are not deleted. Use explicit metadata/tag/link commands where supported, such as `update --depth`, rather than assuming another reindex fixes every mismatch.

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

- The command does not enforce a clean working tree. Its path list comes from committed refs, but content is read from the working tree: uncommitted content on selected paths can be imported, while edits on unselected paths are missed.
- If `ORIG_HEAD` is stale (no recent pull/merge), pass `--since` explicitly.
- Out-of-band body edits need `reindex` or `watch`; metadata-only changes need the appropriate explicit mutation command, not a promised full metadata refresh.
- Rename/copy statuses are skipped. A `reindex` can discover projection paths and clean up orphans, but retains the existing-record metadata and link limits described above. Do not delete the derived index as a recovery step.
- Individual read/parse/missing-ID failures are printed as `SKIP` and counted in the summary, but can still leave exit status 0. Inspect the summary before treating sync as complete.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan reindex`](/docs/cli/reindex/) - projection scan, body sync, and orphan cleanup
- [`forgeplan watch`](/docs/cli/watch/) - live sync daemon
- [`forgeplan health`](/docs/cli/health/) - verify after sync
