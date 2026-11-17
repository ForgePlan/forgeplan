---
title: forgeplan remember
description: "Save a short local memory for later recall, or manage memories with --list and --forget. This is Forgeplan core memory, not Hindsight: it does not configure a bank or write to a Hindsight server."
---

Save a short local memory for later `recall`, or manage memories with `--list` and `--forget`. This is Forgeplan core memory, not Hindsight: it does not configure a bank or write to a Hindsight server.

## Usage

```text
forgeplan remember [OPTIONS] [TEXT]
```

## Arguments

```text
  [TEXT]  Text to remember (omit for --list or --forget)
```

## Options

```text
  -c, --category <CATEGORY>  Memory category: fact, convention, procedure, insight
      --list                 List all memories
      --forget <FORGET>      Forget (delete) a memory by ID
  -h, --help                 Print help
  -V, --version              Print version
```

## Categories and storage

Documented categories are `fact` (default), `convention`, `procedure`, and `insight`. Use `--category`, not `--kind`. IDs are generated from the text with a `mem-` prefix; copy the returned ID for later operations. Markdown lives in `.forgeplan/memory/` and is indexed in LanceDB. Memories are created active without `valid_until`; no automatic expiry is assigned.

## Examples

```bash
forgeplan remember "always pull dev before creating a feature branch" --category procedure
forgeplan remember "watch drops events on NFS mounts" --category insight
forgeplan remember "release branches use merge commits" --category convention
forgeplan remember --list
forgeplan recall "branch workflow" --category procedure
```

## Typical workflow

Capture a fact, retrieve it with `recall`, then use `promote MEMORY_ID --kind note` when it deserves an artifact. Replace `MEMORY_ID` with an ID from `remember --list`. Promotion requires an explicit target kind; categories do not choose it automatically. `remember --forget MEMORY_ID` deletes the memory. After direct markdown edits, run `reindex`.

`--forget` checks that the ID exists and has kind `memory`, then removes its
projection and store record without a confirmation prompt or undo receipt.
Use it only for the memory you intend to delete. If `--list` is also supplied,
listing takes precedence; otherwise `--forget` takes precedence over text.
Categories are stored as supplied, not restricted to the documented names.
The text-derived ID is not a fresh sequence; similar prefixes can collide.

## See also

- [CLI overview](/docs/cli/)
- [`forgeplan recall`](/docs/cli/recall/) - search and retrieve memories
- [`forgeplan promote`](/docs/cli/promote/) - upgrade a memory into an artifact
- [`forgeplan journal`](/docs/cli/journal/) - full decision journal with R_eff
- [`forgeplan new note`](/docs/cli/new/) - go straight to a Note if the memory is already mature
