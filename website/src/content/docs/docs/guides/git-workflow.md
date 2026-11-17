---
title: Git Workflow
description: Branching strategy and commit conventions for Forgeplan projects
---

These branch and review conventions describe development of Forgeplan itself.
Other projects can choose their own branches; the CLI does not enforce this
branching strategy.

## Branching Strategy

```
main                    ← production (tagged releases)
  │
dev                     ← integration branch
  ├── feat/prd-018-dag  ← feature branch
  ├── fix/search-bug    ← bugfix branch
  └── docs/rfc-002      ← docs-only branch
```

## Branch Rules

| Branch | Created from | Merges into | Strategy |
|--------|-------------|-------------|----------|
| `feat/*`, `fix/*` | **dev** | **dev** | Merge commit via PR |
| `release/v0.x.0` | **dev** | **main** + **dev** | Merge commit |
| `hotfix/*` | **main** | **main** + **dev** | Cherry-pick |

## Commit Format

```
<type>(<scope>): <description>

[body - what and why]

Refs: RFC-001, FR-001..004
```

### Types

| Type | When |
|------|------|
| `feat` | New functionality |
| `fix` | Bug fix |
| `docs` | Documentation / artifacts |
| `refactor` | No behavior change |
| `test` | Tests only |
| `chore` | Build, deps, CI |

## PR Pipeline

```
Code → Audit → Fix → Test → Fmt → Lint → Verify → PR
```

**Never create a PR immediately after code.** Verify the changed user-facing
surfaces with the actual CLI/MCP client as well as the relevant tests. Passing
formatting and compilation alone does not establish end-to-end behavior.

## Before every commit

```bash
cargo fmt           # format
cargo fmt -- --check  # verify: 0 diffs
cargo check         # compile: 0 warnings
cargo test          # all pass
```

## Keep the artifact index in sync with Git

Commit artifact markdown, configuration, and links, not the derived
`.forgeplan/lance/` database. Each clone has its own index. After a fresh clone,
use `forgeplan reindex` to rebuild it from the checked-out markdown.

In an initialized Git repository, install the optional sync hooks without
downloading the model:

```bash
forgeplan setup --skip-model --skip-alias
```

Setup installs or refreshes Forgeplan-owned `post-merge` and `post-checkout`
hooks at Git's resolved hooks path (including `core.hooksPath`). It leaves
foreign hooks untouched and reports them. `--skip-hooks` opts out.

- `post-merge` calls `forgeplan git-sync`, using `ORIG_HEAD` as the previous ref.
- `post-checkout` passes the previous HEAD through `--since` on branch switches.
- Hooks do **not** run `embed`, suppress sync output/errors, and do not fail
  the Git operation. A successful pull is not proof that indexing succeeded.

For visible recovery after a pull/merge:

```bash
forgeplan git-sync
forgeplan health
```

For another Git operation, use a known ref from before the change, for example
`forgeplan git-sync --since HEAD@{1}` when that reflog entry is the previous
checkout. Without `--since`, Git sync requires `ORIG_HEAD`.

Git sync handles added, body-modified, and deleted artifact files. In 0.38 it
skips rename/copy entries and may skip metadata/link-only edits when the body
is unchanged. Use `reindex` for file/index recovery, but do not rely on either
path to apply metadata-only changes to an existing record; use the appropriate
`update` or lifecycle command. `scan-import` imports new documents and is not
an index repair command.

Run `forgeplan embed` separately after synchronized body changes if you need
semantic search. It encodes missing/changed bodies and can download the
~2.1 GB BGE-M3 model if it is not already cached.

## Release branch synchronization

After a `release/v* → main` merge, open a sync PR from the released `main`
state back into `dev` (for example `chore/sync-main-to-dev-after-v0.38.0`).
Do not push directly to protected branches or continue pushing to a feature
branch whose PR has already merged.
