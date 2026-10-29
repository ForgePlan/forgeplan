---
title: forgeplan setup
description: "One-time per-machine preparation: create the fpl alias and download the embedding model"
---

`forgeplan setup` prepares the `fpl` alias and model and, inside a Git repository, installs sync hooks unless `--skip-hooks` is passed. Without a model, smart search can use keyword ranking; forced `--semantic` can fail. Setup does not embed workspace artifacts or enable persistent automatic indexing.

The alias exists because the two installation paths land in different states. Homebrew and `install.sh` binaries are produced by cargo-dist, which creates the alias through its `bin-aliases` setting. `cargo install` has no equivalent and **no post-install hook at all**, so a source install has `forgeplan` and no `fpl`.

Model weights are not included in the binary. Prepare them deliberately through an appropriate `setup`/`embed` path. The selected semantic-read path uses the local cache rather than downloading a model on the first search.

## When to use

- After installation when model preparation or hooks are wanted. Release builds include semantic support; source installs need `--features semantic-search`.
- Once per machine - both the alias and the model cache are machine-wide, not per project.
- After moving or reinstalling the binary, to point the alias at the new location.
- Before going offline, if you expect to use semantic search and have not fetched the model yet.

## When NOT to use

- On a Homebrew or `install.sh` binary when you only want the alias — those installs already have `fpl` from cargo-dist. You would still want the model, so pass `--skip-alias` rather than skipping the command.
- In CI. Runners rarely benefit from a 2.1 GB download; if you want only the alias there, pass `--skip-model`.
- As a substitute for `forgeplan init` - this prepares the machine, not a workspace.

## Usage

```text
forgeplan setup [OPTIONS]
```

## Options

```text
      --skip-model  Skip the embedding-model download
      --skip-alias  Skip creating the `fpl` alias
      --skip-hooks  Skip installing project Git sync hooks
  -h, --help        Print help
  -V, --version     Print version
```

## Examples

### Example 1: First-install sequence without Git hook installation

```bash
cargo install --git https://github.com/ForgePlan/forgeplan --features semantic-search
forgeplan setup --skip-hooks
forgeplan init
```

Prepares the machine, then creates a workspace. The model download shows a progress bar and runs once; later projects reuse the same cache.

### Example 2: Alias only, no download

```bash
forgeplan setup --skip-model --skip-hooks
```

Useful on a metered connection, or when you only want the short `fpl` command and are content with keyword search for now. Run `forgeplan setup` again later to fetch the model.

### Example 3: Model only, leave PATH alone

```bash
forgeplan setup --skip-alias --skip-hooks
```

For anyone who already has their own `fpl` on PATH, or manages shims through their own tooling.

## What the alias step does

The symlink is created **next to the binary that is running**, derived from the executable's own path rather than a guessed `~/.cargo/bin`. That matters when several installations exist: the alias always points at the binary you actually invoked, not a different copy elsewhere on PATH.

An existing `fpl` is **never overwritten**. If a file already occupies that path and it is not our symlink, the command reports it and moves on:

```text
! Not touching /Users/you/.cargo/bin/fpl — something is already there.
  Remove it first if you want the alias.
```

Symlinks are created on Unix-like systems. On Windows the step is skipped with a note, since unprivileged symlinks are not dependable there.

## What the model step does

The selected build uses tract, not fastembed. Models default to
`~/.forgeplan/models` on macOS, Linux and Windows. `FORGEPLAN_MODEL_CACHE`
is the explicit override; `HF_HOME` does not override this download path.
Old platform caches remain in place and can be selected explicitly.

A prepared model is only one prerequisite. Semantic answers need eligible vectors.
Automatic maintenance additionally needs a compatible build, active configuration
and a running handler with maintenance capability.

A failed download is not a failed setup. The alias may well have been created, and everything except semantic search keeps working, so the command reports the problem and exits successfully rather than aborting.

## Relationship to `forgeplan init`

`forgeplan init` offers the same model download when run interactively, so a user who starts with `init` is not left to discover this command. The three paths differ on purpose:

| Invocation | Model download |
|---|---|
| `forgeplan init` (interactive) | asks, defaulting to **no** |
| `forgeplan init --with-model` | yes, without asking |
| `forgeplan init -y` | no, unless `--with-model` is explicitly supplied |

An explicit `--with-model` differs from ordinary `-y`. If `init` returns early because a workspace already exists, model preparation may not run at all.

The alias step lives only here, not in `init`, because it is a machine-level concern rather than a per-workspace one.

## See also

- [`forgeplan init`](/docs/cli/init/) - create a workspace; also offers the model download
- [`forgeplan embed`](/docs/cli/embed/) - generate embeddings for artifacts
- [`forgeplan setup-skill`](/docs/cli/setup-skill/) - install the `/forge` Claude Code skill
- [Installation](/docs/getting-started/installation/) - which builds carry semantic search, and why
- [Configuration](/docs/getting-started/configuration/) - the `embedding:` config block

<!-- forgeplan-runtime:start -->
## Captured CLI help

Source: `forgeplan 0.39.0+constellation.s1.1`, `forgeplan setup --help`.

```text
One-time per-machine preparation: `fpl` alias + embedding model

`cargo install` has no post-install hook and does not create the `fpl` alias that brew and install.sh get from cargo-dist's `bin-aliases`, so a source install has `forgeplan` but no `fpl`. This creates it, and downloads the ~2.1 GB embedding model up front rather than letting the first semantic search stall for minutes with no explanation.

Idempotent, and neither step is required for ForgePlan to work.

Usage: forgeplan setup [OPTIONS]

Options:
      --skip-model
          Skip the embedding-model download

      --skip-alias
          Skip creating the `fpl` alias

      --skip-hooks
          Skip installing the git hooks that keep the index in step with `git pull` and branch switches (PROB-097)

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```
<!-- forgeplan-runtime:end -->
