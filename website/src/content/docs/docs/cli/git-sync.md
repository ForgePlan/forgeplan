---
title: forgeplan git-sync
description: "Sync artifact changes from git operations (pull/merge) into LanceDB"
---

Incrementally sync LanceDB with markdown changes introduced by a git
operation — typically `git pull`, `git merge`, or `git rebase`. Instead of
rebuilding the entire index, `git-sync` diffs the working tree against a
git reference and only re-parses the artifacts that actually changed.

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

1. Runs `git diff --name-only <SINCE> HEAD` scoped to `.forgeplan/`.
2. Filters the result to artifact `.md` files.
3. For each changed file: re-parses and updates its LanceDB row.
4. For deleted files: removes the corresponding rows from the index.
5. Recomputes derived fields (tags, links, scoring) for the touched set.

`ORIG_HEAD` is the default because `git pull`/`git merge`/`git rebase` all
set it to the previous tip of the current branch, giving `git-sync` an exact
"what this pull brought in" window.

## When to use it

- **After `git pull`** — teammates merged new PRDs/RFCs into `dev`, you just
  pulled, and you want them in your local LanceDB. `git-sync` runs in a
  fraction of the time of a full reindex.
- **After `git merge feature/xyz`** — same idea for local merges.
- **After `git checkout`** between branches with divergent artifact state —
  pass `--since <other-branch>` to sync the diff.

## Remote team workflow

```bash
# Morning routine with an active team
git checkout dev
git pull origin dev          # pulls 3 new PRDs from teammates
forgeplan git-sync           # <1s: only the 3 new files re-indexed
forgeplan list --since 1d    # see what arrived
forgeplan health             # check for new blind spots
```

Compare against a cold rebuild:

```bash
forgeplan reindex            # walks the whole workspace — safe but slower
```

For small workspaces the difference is negligible; for workspaces with
hundreds of artifacts `git-sync` is substantially faster.

## `git-sync` vs `reindex` vs `watch`

| Command     | Use when                                         | Cost              |
|-------------|--------------------------------------------------|-------------------|
| `git-sync`  | After git pull/merge, know the ref to diff from  | O(changed files)  |
| `reindex`   | Manual edits, fresh clone, unknown drift         | O(all artifacts)  |
| `watch`     | Interactive editing, want live sync              | daemon, O(event)  |

**Rule of thumb:** if git just moved HEAD, use `git-sync`. If you edited
files yourself in an editor, use `reindex` (or leave `watch` running). If
you're unsure whether the index is in sync, `reindex` is always the safe
fallback.

## Example

```bash
# Default — since last pull/merge
forgeplan git-sync

# Explicit diff base (e.g. against main)
forgeplan git-sync --since origin/main

# Check what a PR branch brought in
git checkout feat/prd-050-discover
forgeplan git-sync --since dev
```

## Limitations

- Requires a clean git working tree for reliable diffing — uncommitted
  edits may be skipped or double-counted. Pair with `watch` for dirty trees.
- If `ORIG_HEAD` is stale (no recent pull/merge), pass `--since` explicitly.
- Doesn't detect out-of-band markdown edits — use `reindex` for those.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan reindex`](/docs/cli/reindex/) — full rebuild fallback
- [`forgeplan watch`](/docs/cli/watch/) — live sync daemon
- [`forgeplan health`](/docs/cli/health/) — verify after sync
