---
title: forgeplan setup
description: "One-time per-machine preparation: create the fpl alias and download the embedding model"
---

`forgeplan setup` does the two things a `cargo install` cannot do for itself: it creates the **`fpl` alias** and downloads the **embedding model** used by semantic search. Both steps are idempotent, and neither is required for Forgeplan to work - without the model, search falls back to BM25 keyword ranking; without the alias, `forgeplan` still runs under its full name.

The alias exists because the two installation paths land in different states. Homebrew and `install.sh` binaries are produced by cargo-dist, which creates the alias through its `bin-aliases` setting. `cargo install` has no equivalent and **no post-install hook at all**, so a source install has `forgeplan` and no `fpl`.

The model download applies to **every** install. Semantic search is compiled into all released binaries (ADR-023), but the 2.1 GB of BGE-M3 weights are not — they are a lazy first-use download, and fetching them deliberately beats discovering the wait mid-task when the first semantic search appears to hang.

## When to use

- Immediately after any install — Homebrew, `install.sh`, a GitHub Release archive or `cargo install`. All of them ship semantic search; none of them ship the model.
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
  -h, --help        Print help
  -V, --version     Print version
```

## Examples

### Example 1: Full first-install sequence

```bash
cargo install --git https://github.com/ForgePlan/forgeplan --features semantic-search
forgeplan setup
forgeplan init
```

Prepares the machine, then creates a workspace. The model download shows a progress bar and runs once; later projects reuse the same cache.

### Example 2: Alias only, no download

```bash
forgeplan setup --skip-model
```

Useful on a metered connection, or when you only want the short `fpl` command and are content with keyword search for now. Run `forgeplan setup` again later to fetch the model.

### Example 3: Model only, leave PATH alone

```bash
forgeplan setup --skip-alias
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

Constructing the embedder triggers the download; fastembed prints its own progress bar. The model is cached machine-wide:

| Platform | Cache location |
|---|---|
| macOS | `~/Library/Caches/forgeplan/models` |
| Linux | `~/.cache/forgeplan/models` |
| Windows | `%LOCALAPPDATA%\forgeplan\models` |

Override with `FORGEPLAN_MODEL_CACHE`. If `HF_HOME` is set it takes precedence over both - that is fastembed's behaviour, deliberately not overridden so a shared HuggingFace cache stays authoritative.

A failed download is not a failed setup. The alias may well have been created, and everything except semantic search keeps working, so the command reports the problem and exits successfully rather than aborting.

## Relationship to `forgeplan init`

`forgeplan init` offers the same model download when run interactively, so a user who starts with `init` is not left to discover this command. The three paths differ on purpose:

| Invocation | Model download |
|---|---|
| `forgeplan init` (interactive) | asks, defaulting to **no** |
| `forgeplan init --with-model` | yes, without asking |
| `forgeplan init -y` | **never** |

`-y` never downloads because agents and CI runners call it routinely; pulling gigabytes because someone ran `init -y` in a container would be a denial of service on their build. The interactive default is also no - wanting keyword search only should not require knowing a flag to avoid a 2 GB transfer.

The alias step lives only here, not in `init`, because it is a machine-level concern rather than a per-workspace one.

## See also

- [`forgeplan init`](/docs/cli/init/) - create a workspace; also offers the model download
- [`forgeplan embed`](/docs/cli/embed/) - generate embeddings for artifacts
- [`forgeplan setup-skill`](/docs/cli/setup-skill/) - install the `/forge` Claude Code skill
- [Installation](/docs/getting-started/installation/) - which builds carry semantic search, and why
- [Configuration](/docs/getting-started/configuration/) - the `embedding:` config block
